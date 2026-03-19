// src/lib.rs
//! Self-Monitoring Clean Architecture Template
//!
//! このライブラリは、Clean ArchitectureとCQRSパターンを実装し、
//! 各層のパフォーマンスをリアルタイムで監視できる自己監視型アプリケーションです。

// ===== Shared Components =====
pub mod shared {
    pub mod error {
        pub mod application_error;
        pub mod domain_error;
        pub mod infrastructure_error;
        pub mod presentation_error;

        // Re-exports for convenience
        // pub use application_error::*;
        // pub use domain_error::*;
        // pub use infrastructure_error::*;
        // pub use presentation_error::*;
    }

    pub mod metrics {
        pub mod analyzer;
        pub mod collector;
        pub mod types;

        // pub use analyzer::*;
        // pub use collector::*;
        // pub use types::*;
    }

    pub mod middleware {
        pub mod auth_middleware;
        pub mod cors_middleware;
        pub mod discord_middleware;
        pub mod metrics_middleware;
        pub mod security_headers_middleware;
        pub mod watch_middleware;

        // pub use auth_middleware::*;
        // pub use cors_middleware::*;
        // pub use metrics_middleware::*;
        // pub use watch_middleware::*;
    }

    pub mod utils {
        pub mod date_time_utils;
        pub mod password_hasher;
        pub mod uuid_generator;

        // pub use date_time_utils::*;
        // pub use password_hasher::*;
        // pub use uuid_generator::*;
    }

    pub mod notification {
        pub mod discord_notification;

        // pub use discord_notification::*;
    }
}

// ===== Domain Layer =====
pub mod domain {
    pub mod entity {
        pub mod user;

        // pub use user::*;
    }

    pub mod value_object {
        pub mod birth_date;
        pub mod email;
        pub mod pagination;
        pub mod password;
        pub mod phone;
        pub mod user_id;
        pub mod user_name;

        pub use birth_date::*;
        pub use email::*;
        pub use pagination::*;
        pub use password::*;
        pub use phone::*;
        pub use user_id::*;
        pub use user_name::*;
    }

    pub mod repository {
        pub mod user_command_repository;
        pub mod user_query_repository;

        // pub use user_command_repository::*;
        // pub use user_query_repository::*;
    }

    pub mod service {
        pub mod id_generator;
        pub mod user_domain_service;

        // pub use user_domain_service::*;
    }
}

// ===== Application Layer =====
pub mod application {
    pub mod dto {
        pub mod user_command_dto;
        pub mod user_request_dto;
        pub mod user_response_dto;

        // pub use user_command_dto::*;
        // pub use user_request_dto::*;
        // pub use user_response_dto::*;
    }

    pub mod commands {
        pub mod create_user_command;
        pub mod delete_user_command;

        // pub use create_user_command::*;
        // pub use delete_user_command::*;
    }

    pub mod queries {
        pub mod get_user_query;
        pub mod list_users_query;
        pub mod search_users_query;

        // pub use get_user_query::*;
        // pub use list_users_query::*;
        // pub use search_users_query::*;
    }

    pub mod usecases {
        pub mod create_user_usecase;
        pub mod delete_user_usecase;
        pub mod get_user_usecase;
        pub mod list_users_usecase;
        pub mod login_usecase;
        pub mod update_user_usecase;

        // pub use create_user_usecase::*;
        // pub use delete_user_usecase::*;
        // pub use get_user_usecase::*;
        // pub use list_users_usecase::*;
        // pub use login_usecase::*;
        // pub use update_user_usecase::*;
    }

    pub mod decorators {
        pub mod metrics_decorator;

        // pub use metrics_decorator::*;
    }
}

// ===== Infrastructure Layer =====
pub mod infrastructure {
    pub mod repository {
        pub mod in_memory_user_command_repository;
        pub mod in_memory_user_query_repository;
        pub mod monitored_repository;

        pub use in_memory_user_command_repository::*;
        pub use in_memory_user_query_repository::*;
        // pub use monitored_repository::*;
    }

    pub mod cqrs {
        pub mod command_store;
        pub mod query_store;
        pub mod synchronizer;

        // pub use command_store::*;
        // pub use query_store::*;
        // pub use synchronizer::*;
    }

    pub mod cache {
        pub mod cache_metrics;
        pub mod lru_cache;

        // pub use cache_metrics::*;
        // pub use lru_cache::*;
    }

    pub mod config {
        pub mod app_config;
        pub mod metrics_config;

        // pub use app_config::*;
        // pub use metrics_config::*;
    }
    pub mod database {
        pub mod sqlite_connection;
    }

    pub mod di {
        pub mod container;
    }

    pub mod web {
        pub mod run;
    }

    pub mod grpc {
        pub mod hello_service;
        pub mod server;
    }

    pub mod utils {
        pub mod cors_settings;
        pub mod graceful_shutdown;
    }
}

// ===== Presentation Layer =====
pub mod presentation {
    pub mod controller {
        pub mod auth_controller;
        pub mod fortune_controller;
        pub mod health_controller;
        pub mod metrics_controller;
        pub mod user_controller;

        // pub use auth_controller::*;
        // pub use health_controller::*;
        // pub use metrics_controller::*;
        // pub use user_controller::*;
    }

    pub mod dto {
        pub mod api_response;
        pub mod create_user_request;
        pub mod delete_user_request;
        pub mod login_request;
        pub mod login_response;
        pub mod metrics_response;
        pub mod update_user_request;
        pub mod user_response;

        // pub use api_response::*;
        // pub use create_user_request::*;
        // pub use delete_user_request::*;
        // pub use login_request::*;
        // pub use login_response::*;
        // pub use metrics_response::*;
        // pub use update_user_request::*;
        // pub use user_response::*;
    }

    pub mod router {
        pub mod app_router;
        pub mod auth_router;
        pub mod fortune_router;
        pub mod grpc_router;
        pub mod metrics_router;
        pub mod user_router;

        // pub use app_router::*;
        // pub use auth_router::*;
        // pub use metrics_router::*;
        // pub use user_router::*;
    }
}

// ===== Application State =====
pub mod state {
    pub mod app_state;

    // pub use app_state::*;
}

// ===== Public API Re-exports =====
// よく使用される型の再エクスポート
// pub use application::dto::{CreateUserRequestDto, UserResponseDto};
// pub use domain::entity::User;
// pub use domain::value_object::{Email, Password, UserId, UserName};
// pub use presentation::dto::{ApiResponse, CreateUserRequest, UserResponse};
// pub use shared::error::{
// //     ApplicationResult, DomainResult, InfrastructureResult, PresentationResult,
// // };
// pub use shared::metrics::{MetricsCollector, end_measurement, start_measurement};
// pub use state::AppState;
// pub use presentation::dto::{ApiResponse, CreateUserRequest, UserResponse};
// pub use shared::error::{
// //     ApplicationResult, DomainResult, InfrastructureResult, PresentationResult,
// // };
// pub use shared::metrics::{MetricsCollector, end_measurement, start_measurement};
// pub use state::AppState;
// pub use presentation::dto::{ApiResponse, CreateUserRequest, UserResponse};
// pub use shared::error::{
// //     ApplicationResult, DomainResult, InfrastructureResult, PresentationResult,
// // };
// pub use shared::metrics::{MetricsCollector, end_measurement, start_measurement};
// pub use state::AppState;
// pub use presentation::dto::{ApiResponse, CreateUserRequest, UserResponse};
// pub use shared::error::{
// //     ApplicationResult, DomainResult, InfrastructureResult, PresentationResult,
// // };
// pub use shared::metrics::{MetricsCollector, end_measurement, start_measurement};
// pub use state::AppState;
