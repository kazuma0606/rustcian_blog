output "id" {
  description = "Resource ID of the Storage Account."
  value       = azurerm_storage_account.main.id
}

output "account_name" {
  description = "Name of the Storage Account."
  value       = azurerm_storage_account.main.name
}

output "table_endpoint" {
  description = "Azure Table Storage REST endpoint."
  value       = azurerm_storage_account.main.primary_table_endpoint
}

output "primary_access_key" {
  description = "Primary access key for the Storage Account."
  value       = azurerm_storage_account.main.primary_access_key
  sensitive   = true
}
