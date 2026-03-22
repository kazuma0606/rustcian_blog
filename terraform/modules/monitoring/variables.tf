variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  type = string
}

variable "slack_webhook_url" {
  description = "Slack incoming webhook URL for monitor alerts. Leave empty to disable."
  type        = string
  default     = ""
  sensitive   = true
}
