terraform {
  required_version = ">= 1.7"

  required_providers {
    azurerm = {
      source  = "hashicorp/azurerm"
      version = "~> 4.0"
    }
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
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  model_capacity      = var.openai_model_capacity
}

module "comms" {
  source              = "./modules/comms"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
}

module "app" {
  source              = "./modules/app"
  resource_group_name = azurerm_resource_group.main.name
  location            = azurerm_resource_group.main.location
  prefix              = "${var.prefix}-${var.environment}"
  sku_name            = var.app_service_sku
  container_image     = var.container_image
  container_port      = var.container_port

  app_settings = {
    # Storage — blog content is embedded in the container image at build time.
    STORAGE_BACKEND = "local"

    # Table Storage for comments and contact messages.
    # NOTE: The application currently authenticates via SharedKeyLite using the
    # storage account key stored as a Key Vault secret.  A future improvement
    # is to switch to managed-identity auth (Storage Table Data Contributor).
    AZURITE_TABLE_ENDPOINT = module.storage.table_endpoint

    # Authentication
    ADMIN_AUTH_MODE      = var.admin_auth_mode
    ENTRA_TENANT_ID      = var.entra_tenant_id
    ENTRA_CLIENT_ID      = var.entra_client_id
    ENTRA_ADMIN_GROUP_ID = var.entra_admin_group_id

    # Observability — Application Insights connection string from Key Vault.
    OBSERVABILITY_BACKEND                 = "appinsights"
    APPLICATIONINSIGHTS_CONNECTION_STRING = "@Microsoft.KeyVault(SecretUri=${module.keyvault.app_insights_cs_uri})"

    # Slack notifications from Key Vault.
    SLACK_WEBHOOK_URL = "@Microsoft.KeyVault(SecretUri=${module.keyvault.slack_webhook_url_uri})"

    # Azure OpenAI
    AZURE_OPENAI_ENDPOINT   = module.openai.endpoint
    AZURE_OPENAI_DEPLOYMENT = module.openai.deployment_name

    # OpenAI API key from Key Vault.
    AZURE_OPENAI_API_KEY = "@Microsoft.KeyVault(SecretUri=${module.keyvault.openai_api_key_uri})"

    # Storage account key for Table Storage from Key Vault.
    AZURE_STORAGE_ACCOUNT_NAME = module.storage.account_name
    AZURE_STORAGE_ACCOUNT_KEY  = "@Microsoft.KeyVault(SecretUri=${module.keyvault.storage_account_key_uri})"

    BASE_URL = var.base_url
  }

  key_vault_id = module.keyvault.id
}

# ---------------------------------------------------------------------------
# Grant the App Service's managed identity read access to Key Vault secrets.
# Handled here at root scope to avoid a circular dependency between the
# app module (needs secret URIs) and the keyvault module (needs principal_id).
# ---------------------------------------------------------------------------

resource "azurerm_role_assignment" "app_keyvault_secrets" {
  scope                = module.keyvault.id
  role_definition_name = "Key Vault Secrets User"
  principal_id         = module.app.principal_id
}

# Grant storage access for future managed-identity migration.
resource "azurerm_role_assignment" "app_storage_tables" {
  scope                = module.storage.id
  role_definition_name = "Storage Table Data Contributor"
  principal_id         = module.app.principal_id
}
