output "id" {
  description = "Resource ID of the Key Vault."
  value       = azurerm_key_vault.main.id
}

output "uri" {
  description = "URI of the Key Vault."
  value       = azurerm_key_vault.main.vault_uri
}

output "app_insights_cs_uri" {
  description = "Versioned URI of the Application Insights connection string secret."
  value       = azurerm_key_vault_secret.app_insights_cs.versionless_id
}

output "slack_webhook_url_uri" {
  description = "Versioned URI of the Slack webhook URL secret."
  value       = azurerm_key_vault_secret.slack_webhook_url.versionless_id
}

output "openai_api_key_uri" {
  description = "Versioned URI of the Azure OpenAI API key secret."
  value       = azurerm_key_vault_secret.openai_api_key.versionless_id
}

output "storage_account_key_uri" {
  description = "Versioned URI of the storage account key secret."
  value       = azurerm_key_vault_secret.storage_account_key.versionless_id
}
