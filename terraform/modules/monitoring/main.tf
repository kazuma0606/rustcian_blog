resource "azurerm_log_analytics_workspace" "main" {
  name                = "${var.prefix}-law"
  resource_group_name = var.resource_group_name
  location            = var.location
  sku                 = "PerGB2018"
  retention_in_days   = 30
}

resource "azurerm_application_insights" "main" {
  name                = "${var.prefix}-ai"
  resource_group_name = var.resource_group_name
  location            = var.location
  workspace_id        = azurerm_log_analytics_workspace.main.id
  application_type    = "web"
}

# ---------------------------------------------------------------------------
# Monitor Alerts — Slack webhook action group + ContentError metric alert
# ---------------------------------------------------------------------------

resource "azurerm_monitor_action_group" "slack" {
  count               = var.slack_webhook_url != "" ? 1 : 0
  name                = "${var.prefix}-slack-ag"
  resource_group_name = var.resource_group_name
  short_name          = "slack"

  webhook_receiver {
    name                    = "slack"
    service_uri             = var.slack_webhook_url
    use_common_alert_schema = true
  }
}

resource "azurerm_monitor_metric_alert" "content_error" {
  count               = var.slack_webhook_url != "" ? 1 : 0
  name                = "${var.prefix}-content-error-alert"
  resource_group_name = var.resource_group_name
  scopes              = [azurerm_application_insights.main.id]
  description         = "Alert when ContentError custom metric exceeds threshold"
  severity            = 2
  frequency           = "PT5M"
  window_size         = "PT15M"

  criteria {
    metric_namespace = "microsoft.insights/components"
    metric_name      = "customMetrics/ContentError"
    aggregation      = "Count"
    operator         = "GreaterThan"
    threshold        = 5
  }

  action {
    action_group_id = azurerm_monitor_action_group.slack[0].id
  }
}
