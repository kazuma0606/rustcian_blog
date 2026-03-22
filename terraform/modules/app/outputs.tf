output "hostname" {
  description = "Default hostname of the App Service."
  value       = azurerm_linux_web_app.main.default_hostname
}

output "principal_id" {
  description = "Object ID of the system-assigned managed identity."
  value       = azurerm_linux_web_app.main.identity[0].principal_id
}

output "id" {
  description = "Resource ID of the App Service."
  value       = azurerm_linux_web_app.main.id
}
