output "login_server" {
  description = "ACR login server FQDN (e.g. rustacianprodacr.azurecr.io)."
  value       = azurerm_container_registry.main.login_server
}

output "id" {
  description = "Resource ID of the Container Registry."
  value       = azurerm_container_registry.main.id
}
