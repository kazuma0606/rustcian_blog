variable "resource_group_name" {
  type = string
}

variable "location" {
  type = string
}

variable "prefix" {
  type = string
}

variable "model_capacity" {
  description = "Token-per-minute capacity in thousands for the gpt-4o-mini deployment. Set to 0 to skip deployment creation (useful when quota has not been approved yet)."
  type        = number
  default     = 10
}
