output "container_app_hostname" {
  description = "Default FQDN of the Container App ingress."
  value       = module.app.hostname
}

output "container_app_principal_id" {
  description = "Object ID of the Container App's system-assigned managed identity."
  value       = module.app.principal_id
}

output "storage_table_endpoint" {
  description = "Azure Table Storage REST endpoint."
  value       = module.storage.table_endpoint
}

output "key_vault_uri" {
  description = "URI of the Key Vault."
  value       = module.keyvault.uri
}

output "application_insights_connection_string" {
  description = "Application Insights connection string."
  value       = module.monitoring.connection_string
  sensitive   = true
}

output "openai_endpoint" {
  description = "Azure OpenAI endpoint URL."
  value       = module.openai.endpoint
}

output "openai_deployment_name" {
  description = "Name of the Azure OpenAI model deployment."
  value       = module.openai.deployment_name
}

output "acs_endpoint" {
  description = "Azure Communication Services endpoint URL."
  value       = module.comms.endpoint
}

output "acr_login_server" {
  description = "ACR login server FQDN (use as image prefix in CI)."
  value       = module.registry.login_server
}
