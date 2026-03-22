output "app_service_hostname" {
  description = "Default hostname of the App Service."
  value       = module.app.hostname
}

output "app_service_principal_id" {
  description = "Object ID of the App Service's system-assigned managed identity."
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
