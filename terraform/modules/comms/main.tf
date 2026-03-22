# Azure Communication Services — provisioned but not actively used.
# Reserved for future email notification support (Azure Communication Services Email).
# The resource is created in a disabled state so it can be enabled when needed
# without requiring additional infrastructure changes.

resource "azurerm_communication_service" "main" {
  name                = "${var.prefix}-acs"
  resource_group_name = var.resource_group_name
  data_location       = "Asia Pacific"
}
