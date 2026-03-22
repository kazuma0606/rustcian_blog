variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  type = string
}

variable "sku_name" {
  description = "App Service Plan SKU (e.g. 'B1', 'P1v3')."
  type        = string
  default     = "B1"
}

variable "container_image" {
  description = "Full Docker image reference including tag."
  type        = string
}

variable "container_port" {
  description = "Port the container listens on."
  type        = number
  default     = 8080
}

variable "app_settings" {
  description = "Map of application settings to pass to the web app."
  type        = map(string)
  default     = {}
}

variable "key_vault_id" {
  description = "Resource ID of the Key Vault. Used to enable Key Vault reference resolution."
  type        = string
}
