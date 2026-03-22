# Storage account name must be 3-24 lowercase alphanumeric characters.
# Strip hyphens from the prefix to comply with the naming constraint.
locals {
  account_name = lower(replace("${var.prefix}st", "-", ""))
}

resource "azurerm_storage_account" "main" {
  name                     = local.account_name
  resource_group_name      = var.resource_group_name
  location                 = var.location
  account_tier             = "Standard"
  account_replication_type = "LRS"
  min_tls_version          = "TLS1_2"

  # Disable public blob access; only Table Storage is used.
  allow_nested_items_to_be_public = false
}

resource "azurerm_storage_table" "comments" {
  name                 = "comments"
  storage_account_name = azurerm_storage_account.main.name
}

resource "azurerm_storage_table" "contacts" {
  name                 = "contacts"
  storage_account_name = azurerm_storage_account.main.name
}
