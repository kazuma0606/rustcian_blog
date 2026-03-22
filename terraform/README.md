# Terraform — Rustacian Blog Azure IaC

This directory contains the Terraform configuration that provisions all Azure
resources required to run the Rustacian Blog backend in production.

## Resource overview

| Module       | Resources                                                |
|--------------|----------------------------------------------------------|
| `monitoring` | Log Analytics Workspace, Application Insights            |
| `keyvault`   | Key Vault (RBAC), secrets (AppInsights CS, Slack, OpenAI API key, storage key) |
| `storage`    | Storage Account (LRS), Table Storage (`comments`, `contacts`) |
| `openai`     | Azure OpenAI Cognitive Account, gpt-4o-mini deployment   |
| `comms`      | Azure Communication Services (reserved, not active)      |
| `app`        | App Service Plan, Linux Web App (Docker), system-assigned managed identity |

Root resources: Resource Group, Key Vault RBAC role assignments.

## Prerequisites

- [Terraform ≥ 1.7](https://developer.hashicorp.com/terraform/downloads)
- Azure CLI authenticated: `az login`
- Sufficient permissions on the subscription (Contributor + User Access Administrator)

## First-time setup

```sh
# 1. Copy and fill in the required variable values.
cp terraform.tfvars.example terraform.tfvars

# 2. Initialise providers and modules.
terraform init

# 3. Preview changes.
terraform plan

# 4. Apply.
terraform apply
```

## Required variables (`terraform.tfvars`)

```hcl
container_image = "ghcr.io/<your-org>/rustacian-blog:latest"
base_url        = "https://blog.example.com"
```

Optional overrides (see `variables.tf` for defaults):

```hcl
prefix               = "rustacian"
location             = "japaneast"
environment          = "prod"
app_service_sku      = "P1v3"
admin_auth_mode      = "entra"
entra_tenant_id      = "<tenant-id>"
entra_client_id      = "<client-id>"
entra_admin_group_id = "<group-id>"
```

## Secrets that require manual population

After `terraform apply`, set the following Key Vault secrets via the portal or CLI:

```sh
az keyvault secret set \
  --vault-name "<prefix>-<env>-kv" \
  --name "slack-webhook-url" \
  --value "https://hooks.slack.com/services/..."

az keyvault secret set \
  --vault-name "<prefix>-<env>-kv" \
  --name "azure-openai-api-key" \
  --value "<api-key>"

az keyvault secret set \
  --vault-name "<prefix>-<env>-kv" \
  --name "storage-account-key" \
  --value "$(az storage account keys list \
    --account-name <storage-account-name> \
    --query '[0].value' -o tsv)"
```

The `appinsights-connection-string` secret is populated automatically from the
Application Insights resource during `terraform apply`.

## Destroying resources

```sh
terraform destroy
```

> **Note:** Key Vault soft-delete is enabled (7-day retention). If you need to
> recreate the vault with the same name, purge the soft-deleted vault first:
> `az keyvault purge --name <vault-name>`
