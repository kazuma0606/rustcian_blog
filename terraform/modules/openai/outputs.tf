output "endpoint" {
  description = "Azure OpenAI endpoint URL."
  value       = azurerm_cognitive_account.main.endpoint
}

output "deployment_name" {
  description = "Name of the gpt-4o-mini deployment (empty string if capacity = 0)."
  value       = var.model_capacity > 0 ? azurerm_cognitive_deployment.gpt4o_mini[0].name : "gpt-4o-mini"
}
