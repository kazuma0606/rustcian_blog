resource "azurerm_cognitive_account" "main" {
  name                = "${var.prefix}-oai"
  resource_group_name = var.resource_group_name
  location            = var.location
  kind                = "OpenAI"
  sku_name            = "S0"

  # Public network access is required for the backend service to reach OpenAI.
  public_network_access_enabled = true
}

resource "azurerm_cognitive_deployment" "gpt4o_mini" {
  # count = 0 when model_capacity is 0 (quota not yet approved).
  count                = var.model_capacity > 0 ? 1 : 0
  name                 = "gpt-4o-mini"
  cognitive_account_id = azurerm_cognitive_account.main.id

  model {
    format  = "OpenAI"
    name    = "gpt-4o-mini"
    version = "2024-07-18"
  }

  sku {
    name     = "GlobalStandard"
    capacity = var.model_capacity
  }
}
