resource "azurerm_service_plan" "main" {
  name                = "${var.prefix}-asp"
  resource_group_name = var.resource_group_name
  location            = var.location
  os_type             = "Linux"
  sku_name            = var.sku_name
}

resource "azurerm_linux_web_app" "main" {
  name                = "${var.prefix}-app"
  resource_group_name = var.resource_group_name
  location            = var.location
  service_plan_id     = azurerm_service_plan.main.id

  # System-assigned managed identity for Key Vault access.
  identity {
    type = "SystemAssigned"
  }

  site_config {
    application_stack {
      docker_image_name = var.container_image
    }

    # Health check path — backend exposes /health.
    health_check_path = "/health"
  }

  # Merge caller-supplied app settings with the port declaration.
  app_settings = merge(var.app_settings, {
    WEBSITES_PORT = tostring(var.container_port)
  })

  # Allow Key Vault references in app settings to resolve automatically.
  key_vault_reference_identity_id = null # use system-assigned identity

  https_only = true

  logs {
    http_logs {
      retention_in_days = 7
    }
  }
}
