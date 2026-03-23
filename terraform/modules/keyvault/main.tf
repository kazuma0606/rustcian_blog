resource "azurerm_key_vault" "main" {
  name                       = "${var.prefix}-kv"
  resource_group_name        = var.resource_group_name
  location                   = var.location
  tenant_id                  = var.tenant_id
  sku_name                   = "standard"
  enable_rbac_authorization  = true
  purge_protection_enabled   = false
  soft_delete_retention_days = 7
}

# Grant the Terraform service principal permission to set secrets during deployment.
resource "azurerm_role_assignment" "admin_secrets_officer" {
  scope                = azurerm_key_vault.main.id
  role_definition_name = "Key Vault Secrets Officer"
  principal_id         = var.admin_object_id
}

# ---------------------------------------------------------------------------
# Secrets
# Application Insights connection string is populated from the monitoring module.
# Other secrets (Slack webhook, OpenAI API key, storage key) are created with a
# placeholder value; set the real values manually via the Azure portal or CLI.
# The lifecycle block prevents Terraform from overwriting manually set values.
# ---------------------------------------------------------------------------

resource "azurerm_key_vault_secret" "app_insights_cs" {
  name         = "appinsights-connection-string"
  value        = var.app_insights_connection_string
  key_vault_id = azurerm_key_vault.main.id

  depends_on = [azurerm_role_assignment.admin_secrets_officer]
}

resource "azurerm_key_vault_secret" "slack_webhook_url" {
  name         = "slack-webhook-url"
  value        = "placeholder"
  key_vault_id = azurerm_key_vault.main.id

  lifecycle {
    ignore_changes = [value]
  }

  depends_on = [azurerm_role_assignment.admin_secrets_officer]
}

resource "azurerm_key_vault_secret" "openai_api_key" {
  name         = "azure-openai-api-key"
  value        = "placeholder"
  key_vault_id = azurerm_key_vault.main.id

  lifecycle {
    ignore_changes = [value]
  }

  depends_on = [azurerm_role_assignment.admin_secrets_officer]
}

resource "azurerm_key_vault_secret" "storage_account_key" {
  name         = "storage-account-key"
  value        = "placeholder"
  key_vault_id = azurerm_key_vault.main.id

  lifecycle {
    ignore_changes = [value]
  }

  depends_on = [azurerm_role_assignment.admin_secrets_officer]
}

resource "azurerm_key_vault_secret" "acs_access_key" {
  name         = "acs-access-key"
  value        = "placeholder"
  key_vault_id = azurerm_key_vault.main.id

  lifecycle {
    ignore_changes = [value]
  }

  depends_on = [azurerm_role_assignment.admin_secrets_officer]
}
