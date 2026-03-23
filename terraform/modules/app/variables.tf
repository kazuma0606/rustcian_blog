variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  type = string
}

variable "log_analytics_workspace_id" {
  description = "Resource ID of the Log Analytics workspace for the Container Apps Environment."
  type        = string
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

variable "container_cpu" {
  description = "vCPU allocation for the container (e.g. 0.5, 1.0)."
  type        = number
  default     = 0.5
}

variable "container_memory" {
  description = "Memory allocation for the container (e.g. '1Gi', '2Gi')."
  type        = string
  default     = "1Gi"
}

variable "env_vars" {
  description = "Map of plain (non-secret) environment variables to inject into the container."
  type        = map(string)
  default     = {}
}

variable "secret_env_vars" {
  description = "Map of env-var-name -> Key Vault secret URI. Each entry is exposed as a secret-backed environment variable."
  type        = map(string)
  default     = {}
  sensitive   = true
}

variable "key_vault_id" {
  description = "Resource ID of the Key Vault. The module grants Key Vault Secrets User to the Container App's managed identity."
  type        = string
}

variable "acr_login_server" {
  description = "Login server hostname for the Azure Container Registry (e.g. myacr.azurecr.io). Used to configure managed-identity pull access."
  type        = string
  default     = ""
}
