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
  description = "Token-per-minute capacity in thousands for the gpt-4o-mini deployment."
  type        = number
  default     = 10
}
