output "hostname" {
  description = "Default FQDN of the Container App ingress."
  value       = azurerm_container_app.main.latest_revision_fqdn
}

output "principal_id" {
  description = "Object ID of the Container App's system-assigned managed identity."
  value       = azurerm_container_app.main.identity[0].principal_id
}

output "id" {
  description = "Resource ID of the Container App."
  value       = azurerm_container_app.main.id
}
