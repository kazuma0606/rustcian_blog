terraform {
  required_version = ">= 1.7"

  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 4.0"
    }
  }

  # Remote state stored in Azure Blob Storage.
  # Create the storage account and container before running terraform init:
  #   az group create -n tfstate-rg -l japaneast
  #   az storage account create -n rustaciantfstate -g tfstate-rg --sku Standard_LRS
  #   az storage container create -n tfstate --account-name rustaciantfstate
  #
  # Then run:  terraform init -reconfigure
  backend "azurerm" {
    resource_group_name  = "tfstate-rg"
    storage_account_name = "rustaciantfstate"
    container_name       = "tfstate"
    key                  = "rustacian-blog.tfstate"
  }
}

provider "azurerm" {
  features {
    key_vault {
      # Keep soft-deleted vaults recoverable; don't auto-purge on destroy.
      purge_soft_delete_on_destroy = false
    }
  }
}

data "azurerm_client_config" "current" {}

resource "azurerm_resource_group" "main" {
  name     = "${var.prefix}-${var.environment}-rg"
  location = var.location

  tags = {
    environment = var.environment
    project     = var.prefix
  }
}

# ---------------------------------------------------------------------------
# Modules
# ---------------------------------------------------------------------------

module "monitoring" {
  source              = "./modules/monitoring"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  slack_webhook_url   = var.slack_webhook_url
}

module "keyvault" {
  source              = "./modules/keyvault"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  tenant_id           = data.azurerm_client_config.current.tenant_id
  # Allow the Terraform service principal to manage secrets during deployment.
  admin_object_id     = data.azurerm_client_config.current.object_id
  # Pass Application Insights connection string so it can be stored as a secret.
  app_insights_connection_string = module.monitoring.connection_string
}

module "storage" {
  source              = "./modules/storage"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
}

module "openai" {
  source              = "./modules/openai"
  resource_group_name = azurerm_resource_group.main.name
  # Azure OpenAI gpt-4o-mini GlobalStandard quota is available in eastus.
  # japaneast has 0 quota by default on new subscriptions.
  location       = "eastus"
  prefix         = "${var.prefix}-${var.environment}"
  model_capacity = var.openai_model_capacity
}

module "comms" {
  source              = "./modules/comms"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
}

module "registry" {
  source              = "./modules/registry"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  sku                 = var.acr_sku

  # Granted after module.app creates the managed identity.
  container_app_principal_id  = module.app.principal_id
  github_actions_principal_id = var.github_actions_principal_id
}

module "app" {
  source              = "./modules/app"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  container_image     = var.container_image
  container_port      = var.container_port
  container_cpu       = var.container_cpu
  container_memory    = var.container_memory
  key_vault_id        = module.keyvault.id
  acr_login_server    = module.registry.login_server

  log_analytics_workspace_id = module.monitoring.workspace_id

  # Plain (non-secret) environment variables.
  env_vars = {
    # Storage — blog content is served from Azure Blob Storage via Managed Identity.
    STORAGE_BACKEND        = "azurite"
    AZURITE_BLOB_ENDPOINT  = module.storage.blob_endpoint
    STATIC_PUBLISH_BACKEND = "azurite"
    STATIC_PUBLISH_PREFIX  = "site"

    # Table Storage endpoint for comments and contact messages.
    # Authentication uses Managed Identity (Storage Table Data Contributor).
    AZURITE_TABLE_ENDPOINT     = module.storage.table_endpoint
    AZURE_STORAGE_ACCOUNT_NAME = module.storage.account_name

    # Admin authentication.
    ADMIN_AUTH_MODE      = var.admin_auth_mode
    ENTRA_TENANT_ID      = var.entra_tenant_id
    ENTRA_CLIENT_ID      = var.entra_client_id
    ENTRA_ADMIN_GROUP_ID = var.entra_admin_group_id
    ENTRA_REDIRECT_URI   = var.entra_redirect_uri

    # Observability.
    OBSERVABILITY_BACKEND = "appinsights"

    # Azure OpenAI.
    AZURE_OPENAI_ENDPOINT   = module.openai.endpoint
    AZURE_OPENAI_DEPLOYMENT = module.openai.deployment_name

    # Azure Communication Services.
    ACS_ENDPOINT       = module.comms.endpoint
    ACS_SENDER_ADDRESS = var.acs_sender_address

    BASE_URL = var.base_url
  }

  # Key Vault-backed secrets — injected as Container Apps secrets and exposed
  # as environment variables. Values are KV versionless secret URIs.
  secret_env_vars = {
    APPLICATIONINSIGHTS_CONNECTION_STRING = module.keyvault.app_insights_cs_uri
    SLACK_WEBHOOK_URL                     = module.keyvault.slack_webhook_url_uri
    AZURE_OPENAI_API_KEY                  = module.keyvault.openai_api_key_uri
    ACS_ACCESS_KEY                        = module.keyvault.acs_access_key_uri
    AZURE_STORAGE_ACCOUNT_KEY             = module.keyvault.storage_account_key_uri
  }
}

# ---------------------------------------------------------------------------
# Grant the Container App's managed identity access to Azure Table Storage.
# The Key Vault Secrets User role is granted inside the app module.
# ---------------------------------------------------------------------------

resource "azurerm_role_assignment" "app_storage_tables" {
  scope                = module.storage.id
  role_definition_name = "Storage Table Data Contributor"
  principal_id         = module.app.principal_id
}

resource "azurerm_role_assignment" "app_storage_blobs" {
  scope                = module.storage.id
  role_definition_name = "Storage Blob Data Contributor"
  principal_id         = module.app.principal_id
}

# Allow GitHub Actions SP to upload content blobs (content-deploy workflow).
resource "azurerm_role_assignment" "github_storage_blobs" {
  scope                = module.storage.id
  role_definition_name = "Storage Blob Data Contributor"
  principal_id         = var.github_actions_principal_id
}

# ---------------------------------------------------------------------------
# Dev / Staging environment
# Isolated resource group and storage; shares monitoring, keyvault, openai,
# comms, and registry with prod to minimise cost.
# ---------------------------------------------------------------------------

resource "azurerm_resource_group" "dev" {
  name     = "${var.prefix}-dev-rg"
  location = var.location

  tags = {
    environment = "dev"
    project     = var.prefix
  }
}

module "storage_dev" {
  source              = "./modules/storage"
  resource_group_name = azurerm_resource_group.dev.name
  location            = azurerm_resource_group.dev.location
  # Resolves to "rustaciandevst" — meets Azure 3-24 lowercase alphanum constraint.
  prefix = "${var.prefix}-dev"
}

module "app_dev" {
  source              = "./modules/app"
  resource_group_name = azurerm_resource_group.dev.name
  location            = azurerm_resource_group.dev.location
  prefix              = "${var.prefix}-dev"
  container_image     = var.container_image_dev
  container_port      = var.container_port
  # Smaller allocation: dev traffic is minimal.
  container_cpu    = 0.25
  container_memory = "0.5Gi"
  key_vault_id     = module.keyvault.id
  acr_login_server = module.registry.login_server

  log_analytics_workspace_id = module.monitoring.workspace_id

  env_vars = {
    STORAGE_BACKEND        = "azurite"
    AZURITE_BLOB_ENDPOINT  = module.storage_dev.blob_endpoint
    STATIC_PUBLISH_BACKEND = "azurite"
    STATIC_PUBLISH_PREFIX  = "site"

    AZURITE_TABLE_ENDPOINT     = module.storage_dev.table_endpoint
    AZURE_STORAGE_ACCOUNT_NAME = module.storage_dev.account_name

    # Disable Entra auth for dev; admin panel accessible without token.
    ADMIN_AUTH_MODE = "local-dev"

    # Log to stdout in dev; no Application Insights needed.
    OBSERVABILITY_BACKEND = "stdout"

    AZURE_OPENAI_ENDPOINT   = module.openai.endpoint
    AZURE_OPENAI_DEPLOYMENT = module.openai.deployment_name

    ACS_ENDPOINT       = module.comms.endpoint
    ACS_SENDER_ADDRESS = var.acs_sender_address

    BASE_URL = var.base_url_dev
  }

  # Dev uses Managed Identity for storage; only API secrets are needed.
  secret_env_vars = {
    AZURE_OPENAI_API_KEY = module.keyvault.openai_api_key_uri
    ACS_ACCESS_KEY       = module.keyvault.acs_access_key_uri
  }
}

# Grant the dev Container App's managed identity access to dev storage.
resource "azurerm_role_assignment" "dev_app_storage_tables" {
  scope                = module.storage_dev.id
  role_definition_name = "Storage Table Data Contributor"
  principal_id         = module.app_dev.principal_id
}

resource "azurerm_role_assignment" "dev_app_storage_blobs" {
  scope                = module.storage_dev.id
  role_definition_name = "Storage Blob Data Contributor"
  principal_id         = module.app_dev.principal_id
}

# Grant the dev Container App's managed identity pull access to the shared ACR.
resource "azurerm_role_assignment" "dev_app_acr_pull" {
  scope                = module.registry.id
  role_definition_name = "AcrPull"
  principal_id         = module.app_dev.principal_id
}

# Allow GitHub Actions SP to upload dev content blobs.
resource "azurerm_role_assignment" "dev_github_storage_blobs" {
  scope                = module.storage_dev.id
  role_definition_name = "Storage Blob Data Contributor"
  principal_id         = var.github_actions_principal_id
}
