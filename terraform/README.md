# Terraform — Rustacian Blog Azure IaC

This directory contains the Terraform configuration that provisions all Azure
resources required to run the Rustacian Blog backend in production.

## Resource overview

| Module       | Resources                                                                       |
|--------------|---------------------------------------------------------------------------------|
| `monitoring` | Log Analytics Workspace, Application Insights, metric alerts                    |
| `keyvault`   | Key Vault (RBAC), secrets (AppInsights CS, Slack, OpenAI API key, ACS key)      |
| `storage`    | Storage Account (LRS), Table Storage (`comments`, `contacts`)                   |
| `openai`     | Azure OpenAI Cognitive Account, gpt-4o-mini deployment                          |
| `comms`      | Azure Communication Services                                                    |
| `app`        | Container Apps Environment, Container App (scale-to-zero), managed identity     |

Root resources: Resource Group, Storage Table Data Contributor role assignment.

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
base_url        = "https://rustacian-blog.com"
```

Optional overrides (see `variables.tf` for defaults):

```hcl
prefix               = "rustacian"
location             = "japaneast"
environment          = "prod"
container_cpu        = 1.0      # 0.5 for dev, 1.0 for prod
container_memory     = "2Gi"    # "1Gi" for dev, "2Gi" for prod
admin_auth_mode      = "entra"
entra_tenant_id      = "<tenant-id>"
entra_client_id      = "<client-id>"
entra_admin_group_id = "<group-id>"
entra_redirect_uri   = "https://rustacian-blog.com/admin/callback"
acs_sender_address   = "DoNotReply@<your-acs-verified-domain>"
```

## Secrets that require manual population

After `terraform apply`, set the following Key Vault secrets via the portal or CLI:

```sh
VAULT="<prefix>-<env>-kv"

az keyvault secret set --vault-name "$VAULT" \
  --name "slack-webhook-url" \
  --value "https://hooks.slack.com/services/..."

az keyvault secret set --vault-name "$VAULT" \
  --name "azure-openai-api-key" \
  --value "<api-key>"

az keyvault secret set --vault-name "$VAULT" \
  --name "acs-access-key" \
  --value "<base64-encoded-acs-access-key>"
```

The `appinsights-connection-string` secret is populated automatically from the
Application Insights resource during `terraform apply`.

## Container Apps migration notes

> Migrated from App Service (Linux Web App) to Azure Container Apps in Phase 8.

### Key differences from App Service

| Aspect | App Service | Container Apps |
|--------|-------------|----------------|
| Key Vault refs | `@Microsoft.KeyVault(SecretUri=...)` in app settings | `secret` block with `key_vault_secret_id` + `env.secret_name` |
| Scale to zero | Not supported on B1/F1 | Native (`min_replicas = 0`) |
| Ingress | `WEBSITES_PORT` env var | `ingress.target_port` in HCL |
| KV role | Root-level role assignment | Inside `app` module (grants before container start) |

### First Container Apps deployment

If you are upgrading an existing deployment that used App Service:

```sh
# Remove the old App Service and Service Plan from state before applying,
# or use terraform state mv if you want to preserve history.
terraform state rm module.app.azurerm_linux_web_app.main
terraform state rm module.app.azurerm_service_plan.main

terraform plan   # verify: destroy App Service, create Container App
terraform apply
```

### Scale-to-zero cold start

With `min_replicas = 0`, the container shuts down after ~5 minutes of inactivity.
The first request after idle will experience a cold start (~5–15 seconds).
Set `min_replicas = 1` in `terraform.tfvars` to disable scale-to-zero for
latency-sensitive deployments:

```hcl
# terraform.tfvars
# (add as a module variable override — requires variables.tf addition)
```

### Destroying resources

```sh
terraform destroy
```

> **Note:** Key Vault soft-delete is enabled (7-day retention). If you need to
> recreate the vault with the same name, purge the soft-deleted vault first:
> `az keyvault purge --name <vault-name>`
