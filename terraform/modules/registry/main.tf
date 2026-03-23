resource "azurerm_container_registry" "main" {
  name                = replace("${var.prefix}acr", "-", "") # ACR names: alphanumeric only
  resource_group_name = var.resource_group_name
  location            = var.location
  sku                 = var.sku
  admin_enabled       = false # Use Managed Identity / OIDC, not admin credentials.
}

# Grant the Container App's managed identity pull access to the registry.
resource "azurerm_role_assignment" "container_app_pull" {
  scope                = azurerm_container_registry.main.id
  role_definition_name = "AcrPull"
  principal_id         = var.container_app_principal_id
}

# Grant the GitHub Actions OIDC service principal push access.
resource "azurerm_role_assignment" "github_push" {
  scope                = azurerm_container_registry.main.id
  role_definition_name = "AcrPush"
  principal_id         = var.github_actions_principal_id
}
