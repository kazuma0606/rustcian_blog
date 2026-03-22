output "endpoint" {
  description = "Azure OpenAI endpoint URL."
  value       = azurerm_cognitive_account.main.endpoint
}

output "deployment_name" {
  description = "Name of the gpt-4o-mini deployment."
  value       = azurerm_cognitive_deployment.gpt4o_mini.name
}
