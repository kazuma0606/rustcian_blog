# ---------------------------------------------------------------------------
# Container Apps Environment — connects to the shared Log Analytics workspace.
# ---------------------------------------------------------------------------

resource "azurerm_container_app_environment" "main" {
  name                       = "${var.prefix}-cae"
  resource_group_name        = var.resource_group_name
  location                   = var.location
  log_analytics_workspace_id = var.log_analytics_workspace_id
}

# ---------------------------------------------------------------------------
# Container App — system-assigned managed identity for Key Vault access.
# KV-backed secrets are defined as `secret` blocks and injected via `env`.
# ---------------------------------------------------------------------------

resource "azurerm_container_app" "main" {
  name                         = "${var.prefix}-ca"
  resource_group_name          = var.resource_group_name
  container_app_environment_id = azurerm_container_app_environment.main.id
  revision_mode                = "Single"

  identity {
    type = "SystemAssigned"
  }

  # Each entry in secret_env_vars becomes a Key Vault-backed Container Apps
  # secret. The secret name is derived from the env var name: lowercase with
  # underscores replaced by hyphens (Container Apps naming constraints).
  dynamic "secret" {
    for_each = var.secret_env_vars
    content {
      name                = replace(lower(secret.key), "_", "-")
      key_vault_secret_id = secret.value
      identity            = "System"
    }
  }

  # Use managed identity to pull images from ACR (no static credentials needed).
  dynamic "registry" {
    for_each = var.acr_login_server != "" ? [var.acr_login_server] : []
    content {
      server   = registry.value
      identity = "System"
    }
  }

  template {
    # Scale to zero when idle; allow up to 3 replicas under load.
    min_replicas = 0
    max_replicas = 3

    container {
      name   = "app"
      image  = var.container_image
      cpu    = var.container_cpu
      memory = var.container_memory

      # Plain (non-secret) environment variables.
      dynamic "env" {
        for_each = var.env_vars
        content {
          name  = env.key
          value = env.value
        }
      }

      # Secret-backed environment variables; secret_name must match the
      # derived name used in the `secret` blocks above.
      dynamic "env" {
        for_each = var.secret_env_vars
        content {
          name        = env.key
          secret_name = replace(lower(env.key), "_", "-")
        }
      }

      liveness_probe {
        path                    = "/health"
        port                    = var.container_port
        transport               = "HTTP"
        interval_seconds        = 30
        failure_count_threshold = 3
      }
    }
  }

  ingress {
    external_enabled = true
    target_port      = var.container_port

    traffic_weight {
      percentage      = 100
      latest_revision = true
    }
  }
}

# ---------------------------------------------------------------------------
# Grant the Container App's managed identity read access to Key Vault secrets.
# Placed inside the module so the role is ready before the container starts.
# ---------------------------------------------------------------------------

resource "azurerm_role_assignment" "kv_secrets_user" {
  scope                = var.key_vault_id
  role_definition_name = "Key Vault Secrets User"
  principal_id         = azurerm_container_app.main.identity[0].principal_id
}
