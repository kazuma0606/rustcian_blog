output "id" {
  description = "Resource ID of the Azure Communication Service."
  value       = azurerm_communication_service.main.id
}

output "endpoint" {
  description = "HTTPS endpoint of the Azure Communication Service."
  value       = "https://${azurerm_communication_service.main.name}.communication.azure.com"
}
