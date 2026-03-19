# 📁 Self-Monitoring Clean Architecture
**ディレクトリ構成 & API仕様書**

## 📂 ディレクトリ構成 (lib.rs一括管理)

```
rust-cqrs-clean/
├── Cargo.toml
├── .env.example
├── .gitignore
├── README.md
├── docker-compose.yml
├── scripts/
│   ├── setup.sh
│   └── start.sh
├── static/
│   ├── dashboard.html
│   ├── styles.css
│   └── dashboard.js
├── migrations/
│   └── 001_create_users_table.sql
└── src/
    ├── main.rs                          # アプリケーションエントリーポイント
    ├── lib.rs                           # 全モジュール一括管理
    │
    ├── shared/                          # 共通コンポーネント
    │   ├── error/
    │   │   ├── domain_error.rs          # Domain層エラー定義
    │   │   ├── application_error.rs     # Application層エラー定義
    │   │   ├── infrastructure_error.rs  # Infrastructure層エラー定義
    │   │   └── presentation_error.rs    # Presentation層エラー定義
    │   ├── metrics/
    │   │   ├── collector.rs             # メトリクス収集器
    │   │   ├── types.rs                 # メトリクス型定義
    │   │   └── analyzer.rs              # ボトルネック分析器
    │   ├── middleware/
    │   │   ├── metrics_middleware.rs    # メトリクス収集ミドルウェア
    │   │   ├── cors_middleware.rs       # CORS設定
    │   │   └── auth_middleware.rs       # 認証ミドルウェア
    │   └── utils/
    │       ├── uuid_generator.rs        # UUID生成器
    │       ├── password_hasher.rs       # パスワードハッシュ化
    │       └── date_time_utils.rs       # 日時ユーティリティ
    │
    ├── domain/                          # Domain Layer
    │   ├── entity/
    │   │   └── user.rs                  # User エンティティ + ビジネスロジック
    │   ├── value_object/
    │   │   ├── email.rs                 # Email バリューオブジェクト
    │   │   ├── user_name.rs             # UserName バリューオブジェクト
    │   │   ├── password.rs              # Password バリューオブジェクト
    │   │   ├── phone.rs                 # Phone バリューオブジェクト
    │   │   ├── birth_date.rs            # BirthDate バリューオブジェクト
    │   │   └── user_id.rs               # UserId タイプセーフID
    │   ├── repository/
    │   │   ├── user_command_repository.rs  # Command Repository トレイト
    │   │   └── user_query_repository.rs    # Query Repository トレイト
    │   └── service/
    │       └── user_domain_service.rs   # ドメインサービス
    │
    ├── application/                     # Application Layer
    │   ├── dto/
    │   │   ├── user_request_dto.rs      # HTTP Request DTO
    │   │   ├── user_response_dto.rs     # HTTP Response DTO
    │   │   └── user_command_dto.rs      # Application内部DTO
    │   ├── commands/
    │   │   ├── create_user_command.rs   # ユーザー作成コマンド
    │   │   ├── update_user_command.rs   # ユーザー更新コマンド
    │   │   └── delete_user_command.rs   # ユーザー削除コマンド
    │   ├── queries/
    │   │   ├── get_user_query.rs        # ユーザー取得クエリ
    │   │   ├── list_users_query.rs      # ユーザー一覧クエリ
    │   │   └── search_users_query.rs    # ユーザー検索クエリ
    │   ├── usecases/
    │   │   ├── create_user_usecase.rs   # ユーザー作成ユースケース
    │   │   ├── update_user_usecase.rs   # ユーザー更新ユースケース
    │   │   ├── delete_user_usecase.rs   # ユーザー削除ユースケース
    │   │   ├── get_user_usecase.rs      # ユーザー取得ユースケース
    │   │   ├── list_users_usecase.rs    # ユーザー一覧ユースケース
    │   │   └── login_usecase.rs         # ログインユースケース
    │   └── decorators/
    │       └── metrics_decorator.rs     # メトリクス測定デコレーター
    │
    ├── infrastructure/                  # Infrastructure Layer
    │   ├── repository/
    │   │   ├── in_memory_user_command_repository.rs  # Command Repository実装
    │   │   ├── in_memory_user_query_repository.rs    # Query Repository実装
    │   │   └── monitored_repository.rs               # メトリクス付きRepository
    │   ├── cqrs/
    │   │   ├── command_store.rs         # Write最適化ストア
    │   │   ├── query_store.rs           # Read最適化ストア
    │   │   └── synchronizer.rs          # CQRS同期器
    │   ├── cache/
    │   │   ├── lru_cache.rs             # LRUキャッシュ実装
    │   │   └── cache_metrics.rs         # キャッシュメトリクス
    │   └── config/
    │       ├── app_config.rs            # アプリケーション設定
    │       └── metrics_config.rs        # メトリクス設定
    │
    ├── presentation/                    # Presentation Layer
    │   ├── controller/
    │   │   ├── user_controller.rs       # ユーザー管理エンドポイント
    │   │   ├── auth_controller.rs       # 認証エンドポイント
    │   │   ├── metrics_controller.rs    # メトリクス取得エンドポイント
    │   │   └── health_controller.rs     # ヘルスチェックエンドポイント
    │   ├── dto/
    │   │   ├── create_user_request.rs   # ユーザー作成リクエスト
    │   │   ├── update_user_request.rs   # ユーザー更新リクエスト
    │   │   ├── login_request.rs         # ログインリクエスト
    │   │   ├── user_response.rs         # ユーザーレスポンス
    │   │   ├── login_response.rs        # ログインレスポンス
    │   │   ├── metrics_response.rs      # メトリクスレスポンス
    │   │   └── api_response.rs          # 統一APIレスポンス
    │   └── router/
    │       ├── user_router.rs           # ユーザー関連ルーティング
    │       ├── auth_router.rs           # 認証ルーティング
    │       ├── metrics_router.rs        # メトリクスルーティング
    │       └── app_router.rs            # メインルーター設定
    │
    └── state/
        └── app_state.rs                 # アプリケーション状態管理
```

## 🎯 lib.rs 全モジュール管理

```rust
// src/lib.rs
//! Self-Monitoring Clean Architecture Template
//! 
//! このライブラリは、Clean ArchitectureとCQRSパターンを実装し、
//! 各層のパフォーマンスをリアルタイムで監視できる自己監視型アプリケーションです。

// ===== Shared Components =====
pub mod shared {
    pub mod error {
        pub mod domain_error;
        pub mod application_error;
        pub mod infrastructure_error;
        pub mod presentation_error;
        
        // Re-exports for convenience
        pub use domain_error::*;
        pub use application_error::*;
        pub use infrastructure_error::*;
        pub use presentation_error::*;
    }
    
    pub mod metrics {
        pub mod collector;
        pub mod types;
        pub mod analyzer;
        
        pub use collector::*;
        pub use types::*;
        pub use analyzer::*;
    }
    
    pub mod middleware {
        pub mod metrics_middleware;
        pub mod cors_middleware;
        pub mod auth_middleware;
        
        pub use metrics_middleware::*;
        pub use cors_middleware::*;
        pub use auth_middleware::*;
    }
    
    pub mod utils {
        pub mod uuid_generator;
        pub mod password_hasher;
        pub mod date_time_utils;
        
        pub use uuid_generator::*;
        pub use password_hasher::*;
        pub use date_time_utils::*;
    }
}

// ===== Domain Layer =====
pub mod domain {
    pub mod entity {
        pub mod user;
        
        pub use user::*;
    }
    
    pub mod value_object {
        pub mod email;
        pub mod user_name;
        pub mod password;
        pub mod phone;
        pub mod birth_date;
        pub mod user_id;
        
        pub use email::*;
        pub use user_name::*;
        pub use password::*;
        pub use phone::*;
        pub use birth_date::*;
        pub use user_id::*;
    }
    
    pub mod repository {
        pub mod user_command_repository;
        pub mod user_query_repository;
        
        pub use user_command_repository::*;
        pub use user_query_repository::*;
    }
    
    pub mod service {
        pub mod user_domain_service;
        
        pub use user_domain_service::*;
    }
}

// ===== Application Layer =====
pub mod application {
    pub mod dto {
        pub mod user_request_dto;
        pub mod user_response_dto;
        pub mod user_command_dto;
        
        pub use user_request_dto::*;
        pub use user_response_dto::*;
        pub use user_command_dto::*;
    }
    
    pub mod commands {
        pub mod create_user_command;
        pub mod update_user_command;
        pub mod delete_user_command;
        
        pub use create_user_command::*;
        pub use update_user_command::*;
        pub use delete_user_command::*;
    }
    
    pub mod queries {
        pub mod get_user_query;
        pub mod list_users_query;
        pub mod search_users_query;
        
        pub use get_user_query::*;
        pub use list_users_query::*;
        pub use search_users_query::*;
    }
    
    pub mod usecases {
        pub mod create_user_usecase;
        pub mod update_user_usecase;
        pub mod delete_user_usecase;
        pub mod get_user_usecase;
        pub mod list_users_usecase;
        pub mod login_usecase;
        
        pub use create_user_usecase::*;
        pub use update_user_usecase::*;
        pub use delete_user_usecase::*;
        pub use get_user_usecase::*;
        pub use list_users_usecase::*;
        pub use login_usecase::*;
    }
    
    pub mod decorators {
        pub mod metrics_decorator;
        
        pub use metrics_decorator::*;
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
        pub use monitored_repository::*;
    }
    
    pub mod cqrs {
        pub mod command_store;
        pub mod query_store;
        pub mod synchronizer;
        
        pub use command_store::*;
        pub use query_store::*;
        pub use synchronizer::*;
    }
    
    pub mod cache {
        pub mod lru_cache;
        pub mod cache_metrics;
        
        pub use lru_cache::*;
        pub use cache_metrics::*;
    }
    
    pub mod config {
        pub mod app_config;
        pub mod metrics_config;
        
        pub use app_config::*;
        pub use metrics_config::*;
    }
}

// ===== Presentation Layer =====
pub mod presentation {
    pub mod controller {
        pub mod user_controller;
        pub mod auth_controller;
        pub mod metrics_controller;
        pub mod health_controller;
        
        pub use user_controller::*;
        pub use auth_controller::*;
        pub use metrics_controller::*;
        pub use health_controller::*;
    }
    
    pub mod dto {
        pub mod create_user_request;
        pub mod update_user_request;
        pub mod login_request;
        pub mod user_response;
        pub mod login_response;
        pub mod metrics_response;
        pub mod api_response;
        
        pub use create_user_request::*;
        pub use update_user_request::*;
        pub use login_request::*;
        pub use user_response::*;
        pub use login_response::*;
        pub use metrics_response::*;
        pub use api_response::*;
    }
    
    pub mod router {
        pub mod user_router;
        pub mod auth_router;
        pub mod metrics_router;
        pub mod app_router;
        
        pub use user_router::*;
        pub use auth_router::*;
        pub use metrics_router::*;
        pub use app_router::*;
    }
}

// ===== Application State =====
pub mod state {
    pub mod app_state;
    
    pub use app_state::*;
}

// ===== Public API Re-exports =====
// よく使用される型の再エクスポート
pub use shared::error::{DomainResult, ApplicationResult, InfrastructureResult, PresentationResult};
pub use shared::metrics::{MetricsCollector, start_measurement, end_measurement};
pub use domain::entity::User;
pub use domain::value_object::{Email, UserName, Password, UserId};
pub use application::dto::{CreateUserRequestDto, UserResponseDto};
pub use presentation::dto::{CreateUserRequest, UserResponse, ApiResponse};
pub use state::AppState;
```

## 🌐 API一覧

### Base Configuration
- **Base URL**: `http://localhost:3000`
- **Content-Type**: `application/json`
- **Authentication**: `Bearer <JWT_TOKEN>` (when required)

---

## 👤 User Management APIs

### 1. POST /api/users
**ユーザー作成**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/users` |
| **Auth Required** | No |
| **Content-Type** | `application/json` |

**Request Body:**
```json
{
  "email": "user@example.com",
  "name": "John Doe",
  "password": "SecurePassword123!",
  "phone": "+81-90-1234-5678",      // Optional
  "birth_date": "1990-01-01"        // Optional
}
```

**Response (201 Created):**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "user@example.com",
    "name": "John Doe",
    "phone": "+81-90-1234-5678",
    "birth_date": "1990-01-01",
    "created_at": "2025-07-08T10:30:00Z",
    "updated_at": "2025-07-08T10:30:00Z"
  },
  "message": "User created successfully",
  "request_id": "req_123456",
  "processing_time_ms": 45
}
```

**Possible Errors:**
- `400 Bad Request`: バリデーションエラー
- `409 Conflict`: メールアドレス重複
- `500 Internal Server Error`: サーバーエラー

---

### 2. GET /api/users/{id}
**ユーザー詳細取得**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/users/{id}` |
| **Auth Required** | Yes |
| **Parameters** | `id`: User UUID |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "user@example.com",
    "name": "John Doe",
    "phone": "+81-90-1234-5678",
    "birth_date": "1990-01-01",
    "last_login_at": "2025-07-08T09:15:00Z",
    "created_at": "2025-07-08T08:30:00Z",
    "updated_at": "2025-07-08T09:15:00Z"
  },
  "message": "User retrieved successfully"
}
```

**Possible Errors:**
- `401 Unauthorized`: 認証エラー
- `404 Not Found`: ユーザーが存在しない
- `500 Internal Server Error`: サーバーエラー

---

### 3. GET /api/users
**ユーザー一覧取得**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/users` |
| **Auth Required** | Yes |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `page` | number | 1 | ページ番号 |
| `limit` | number | 20 | 1ページの件数 (最大100) |
| `search` | string | - | 名前・メールでの部分検索 |
| `sort` | string | created_at | ソート項目 (created_at, name, email) |
| `order` | string | desc | ソート順 (asc, desc) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "users": [
      {
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "email": "user1@example.com",
        "name": "John Doe",
        "created_at": "2025-07-08T08:30:00Z"
      },
      {
        "id": "456e7890-e89b-12d3-a456-426614174001",
        "email": "user2@example.com",
        "name": "Jane Smith",
        "created_at": "2025-07-08T09:15:00Z"
      }
    ],
    "pagination": {
      "current_page": 1,
      "total_pages": 5,
      "total_count": 95,
      "per_page": 20,
      "has_next": true,
      "has_prev": false
    }
  },
  "message": "Users retrieved successfully"
}
```

---

### 4. PUT /api/users/{id}
**ユーザー情報更新**

| | |
|---|---|
| **Method** | PUT |
| **Endpoint** | `/api/users/{id}` |
| **Auth Required** | Yes |
| **Parameters** | `id`: User UUID |

**Request Body:**
```json
{
  "name": "Updated Name",           // Optional
  "phone": "+81-90-9876-5432",     // Optional
  "birth_date": "1985-12-25"       // Optional
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "email": "user@example.com",
    "name": "Updated Name",
    "phone": "+81-90-9876-5432",
    "birth_date": "1985-12-25",
    "last_login_at": "2025-07-08T09:15:00Z",
    "created_at": "2025-07-08T08:30:00Z",
    "updated_at": "2025-07-08T11:45:00Z"
  },
  "message": "User updated successfully"
}
```

---

### 5. DELETE /api/users/{id}
**ユーザー削除**

| | |
|---|---|
| **Method** | DELETE |
| **Endpoint** | `/api/users/{id}` |
| **Auth Required** | Yes |
| **Parameters** | `id`: User UUID |

**Response (200 OK):**
```json
{
  "success": true,
  "data": null,
  "message": "User deleted successfully"
}
```

---

## 🔐 Authentication APIs

### 6. POST /api/auth/login
**ユーザーログイン**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/auth/login` |
| **Auth Required** | No |

**Request Body:**
```json
{
  "email": "user@example.com",
  "password": "SecurePassword123!"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "user": {
      "id": "123e4567-e89b-12d3-a456-426614174000",
      "email": "user@example.com",
      "name": "John Doe"
    }
  },
  "message": "Login successful"
}
```

**Possible Errors:**
- `400 Bad Request`: バリデーションエラー
- `401 Unauthorized`: 認証失敗
- `500 Internal Server Error`: サーバーエラー

---

### 7. POST /api/auth/logout
**ユーザーログアウト**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/auth/logout` |
| **Auth Required** | Yes |

**Response (200 OK):**
```json
{
  "success": true,
  "data": null,
  "message": "Logout successful"
}
```

---

### 8. POST /api/auth/refresh
**トークンリフレッシュ**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/auth/refresh` |
| **Auth Required** | Yes |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
    "token_type": "Bearer",
    "expires_in": 3600
  },
  "message": "Token refreshed successfully"
}
```

---

## 📊 Metrics & Monitoring APIs

### 9. GET /api/metrics
**パフォーマンスメトリクス取得**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/metrics` |
| **Auth Required** | Yes (Admin) |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `layer` | string | all | 対象層 (presentation, application, domain, infrastructure) |
| `operation` | string | all | 特定の操作名 |
| `time_range` | string | 1h | 時間範囲 (1h, 24h, 7d) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "performance": {
      "presentation::POST /api/users": {
        "total_calls": 1250,
        "avg_duration_ms": 145,
        "min_duration_ms": 23,
        "max_duration_ms": 2340,
        "p95_duration_ms": 890,
        "p99_duration_ms": 1450,
        "last_call": "2025-07-08T10:29:45Z"
      },
      "application::CreateUserUseCase": {
        "total_calls": 1250,
        "avg_duration_ms": 89,
        "min_duration_ms": 12,
        "max_duration_ms": 1890,
        "p95_duration_ms": 340,
        "p99_duration_ms": 670,
        "last_call": "2025-07-08T10:29:45Z"
      }
    },
    "business": {
      "user_registrations": 1250,
      "user_logins": 4580,
      "failed_logins": 23,
      "feature_usage": {
        "user_profile_view": 2340,
        "user_search": 890
      }
    },
    "resources": {
      "memory_usage_bytes": 52428800,
      "in_memory_records": {
        "users": 1250,
        "sessions": 456
      },
      "cache_hit_rate": 89.5,
      "concurrent_requests": 8
    },
    "real_time": {
      "active_requests": 8,
      "requests_per_second": 12.5,
      "avg_response_time_ms": 145,
      "error_rate_percent": 1.2,
      "uptime_seconds": 86400
    }
  },
  "message": "Metrics retrieved successfully"
}
```

---

### 10. GET /api/metrics/bottlenecks
**ボトルネック分析**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/metrics/bottlenecks` |
| **Auth Required** | Yes (Admin) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "bottlenecks": [
      {
        "layer": "presentation",
        "operation": "POST /api/users",
        "severity": "High",
        "issue_type": "slow_response",
        "current_value": 1450,
        "threshold": 1000,
        "recommendation": "Consider optimization or caching"
      },
      {
        "layer": "infrastructure",
        "operation": "memory_usage",
        "severity": "Medium",
        "issue_type": "high_memory_usage",
        "current_value": 104857600,
        "threshold": 104857600,
        "recommendation": "Consider data cleanup or pagination"
      }
    ],
    "overall_health": {
      "overall": 78.5,
      "performance": 72.0,
      "reliability": 89.0,
      "resource": 75.0
    },
    "recommendations": [
      "Optimize user creation endpoint response time",
      "Implement response caching for frequently accessed data",
      "Consider implementing data archiving for old records"
    ]
  },
  "message": "Bottleneck analysis completed"
}
```

---

### 11. GET /api/metrics/live
**リアルタイムメトリクス (WebSocket)**

| | |
|---|---|
| **Method** | WebSocket |
| **Endpoint** | `/api/metrics/live` |
| **Auth Required** | Yes (Admin) |

**Real-time Data Stream:**
```json
{
  "timestamp": "2025-07-08T10:30:00Z",
  "active_requests": 12,
  "requests_per_second": 15.8,
  "avg_response_time_ms": 156,
  "error_rate_percent": 0.8,
  "memory_usage_mb": 48,
  "latest_requests": [
    {
      "id": "req_789012",
      "method": "POST",
      "path": "/api/users",
      "duration_ms": 123,
      "status": 201,
      "timestamp": "2025-07-08T10:29:58Z"
    }
  ]
}
```

---

## 🏥 Health Check APIs

### 12. GET /health
**ヘルスチェック**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/health` |
| **Auth Required** | No |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "1.0.0",
    "uptime_seconds": 86400,
    "services": {
      "in_memory_store": "healthy",
      "metrics_collector": "healthy",
      "web_server": "healthy"
    },
    "system": {
      "memory_usage_mb": 50,
      "cpu_usage_percent": 15.5
    }
  },
  "message": "System is healthy"
}
```

---

### 13. GET /health/detailed
**詳細ヘルスチェック**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/health/detailed` |
| **Auth Required** | Yes (Admin) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "status": "healthy",
    "version": "1.0.0",
    "uptime_seconds": 86400,
    "build_info": {
      "version": "1.0.0",
      "build_date": "2025-07-08T00:00:00Z",
      "git_commit": "abc123def456",
      "rust_version": "1.80.0"
    },
    "services": {
      "in_memory_store": {
        "status": "healthy",
        "records_count": 1250,
        "memory_usage_mb": 25
      },
      "metrics_collector": {
        "status": "healthy",
        "metrics_count": 156,
        "last_collection": "2025-07-08T10:29:55Z"
      },
      "web_server": {
        "status": "healthy",
        "active_connections": 8,
        "total_requests": 15670
      }
    },
    "performance": {
      "avg_response_time_ms": 145,
      "requests_per_second": 12.5,
      "error_rate_percent": 0.8
    },
    "system": {
      "memory_usage_mb": 50,
      "memory_limit_mb": 512,
      "cpu_usage_percent": 15.5,
      "disk_usage_percent": 45.2,
      "load_average": [0.8, 0.6, 0.4]
    }
  },
  "message": "Detailed system health retrieved"
}
```

---

## 🎨 Dashboard APIs

### 14. GET /dashboard
**メトリクスダッシュボード (HTML)**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/dashboard` |
| **Auth Required** | Yes (Admin) |
| **Content-Type** | `text/html` |

**Response:** リアルタイム更新対応のHTML Dashboard

---

### 15. GET /api/dashboard/data
**ダッシュボード用データ取得**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/dashboard/data` |
| **Auth Required** | Yes (Admin) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "overview": {
      "total_users": 1250,
      "active_sessions": 45,
      "requests_today": 12580,
      "avg_response_time_ms": 145
    },
    "performance_chart": {
      "labels": ["10:25", "10:26", "10:27", "10:28", "10:29"],
      "datasets": [
        {
          "label": "Response Time (ms)",
          "data": [120, 135, 145, 150, 145],
          "borderColor": "rgb(75, 192, 192)"
        },
        {
          "label": "Requests/sec",
          "data": [10, 12, 15, 13, 12],
          "borderColor": "rgb(255, 99, 132)"
        }
      ]
    },
    "layer_performance": {
      "presentation": { "avg_ms": 56, "calls": 1250 },
      "application": { "avg_ms": 43, "calls": 1250 },
      "domain": { "avg_ms": 12, "calls": 1250 },
      "infrastructure": { "avg_ms": 34, "calls": 1250 }
    },
    "error_distribution": {
      "labels": ["Validation", "Not Found", "Auth Failed", "Internal"],
      "data": [15, 8, 3, 2]
    }
  },
  "message": "Dashboard data retrieved"
}
```

---

## 🔍 Search & Filter APIs

### 16. POST /api/users/search
**高度なユーザー検索**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/users/search` |
| **Auth Required** | Yes |

**Request Body:**
```json
{
  "filters": {
    "email_domain": "@company.com",     // Optional
    "name_contains": "john",            // Optional
    "created_after": "2025-01-01",      // Optional
    "created_before": "2025-12-31",     // Optional
    "has_phone": true,                  // Optional
    "age_range": {                      // Optional
      "min": 20,
      "max": 65
    }
  },
  "sort": {
    "field": "created_at",              // created_at, name, email
    "order": "desc"                     // asc, desc
  },
  "pagination": {
    "page": 1,
    "limit": 20
  }
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "users": [
      {
        "id": "123e4567-e89b-12d3-a456-426614174000",
        "email": "john.doe@company.com",
        "name": "John Doe",
        "phone": "+81-90-1234-5678",
        "age": 35,
        "created_at": "2025-07-08T08:30:00Z"
      }
    ],
    "pagination": {
      "current_page": 1,
      "total_pages": 3,
      "total_count": 45,
      "per_page": 20,
      "has_next": true,
      "has_prev": false
    },
    "search_meta": {
      "query_time_ms": 23,
      "filters_applied": 5,
      "cache_hit": false
    }
  },
  "message": "Search completed successfully"
}
```

---

## 📈 Analytics APIs

### 17. GET /api/analytics/summary
**分析サマリー**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/analytics/summary` |
| **Auth Required** | Yes (Admin) |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `period` | string | 24h | 分析期間 (1h, 24h, 7d, 30d) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "time_period": "24h",
    "user_analytics": {
      "new_registrations": 45,
      "active_users": 320,
      "login_success_rate": 94.5,
      "avg_session_duration_minutes": 23
    },
    "performance_analytics": {
      "total_requests": 15670,
      "avg_response_time_ms": 145,
      "error_rate_percent": 1.2,
      "slowest_endpoints": [
        {
          "endpoint": "POST /api/users",
          "avg_time_ms": 245,
          "call_count": 45
        },
        {
          "endpoint": "GET /api/users",
          "avg_time_ms": 178,
          "call_count": 890
        }
      ]
    },
    "resource_analytics": {
      "peak_memory_usage_mb": 78,
      "avg_memory_usage_mb": 52,
      "peak_concurrent_requests": 25,
      "cache_effectiveness": 89.5
    },
    "trends": {
      "registration_trend": "increasing",    // increasing, decreasing, stable
      "performance_trend": "stable",
      "error_trend": "decreasing"
    }
  },
  "message": "Analytics summary retrieved"
}
```

---

### 18. GET /api/analytics/performance/trends
**パフォーマンストレンド分析**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/analytics/performance/trends` |
| **Auth Required** | Yes (Admin) |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `metric` | string | response_time | メトリクス種別 (response_time, throughput, error_rate) |
| `period` | string | 24h | 分析期間 |
| `granularity` | string | hour | データ粒度 (minute, hour, day) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "metric": "response_time",
    "period": "24h",
    "granularity": "hour",
    "data_points": [
      {
        "timestamp": "2025-07-07T11:00:00Z",
        "value": 142,
        "sample_count": 850
      },
      {
        "timestamp": "2025-07-07T12:00:00Z",
        "value": 138,
        "sample_count": 920
      },
      {
        "timestamp": "2025-07-07T13:00:00Z",
        "value": 155,
        "sample_count": 1250
      }
    ],
    "statistics": {
      "min": 125,
      "max": 189,
      "avg": 145,
      "median": 142,
      "p95": 178,
      "p99": 189
    },
    "trend_analysis": {
      "direction": "stable",           // improving, degrading, stable
      "change_percent": 2.1,
      "significance": "low",           // low, medium, high
      "prediction_next_hour": 147
    }
  },
  "message": "Performance trend analysis completed"
}
```

---

## 🚨 Alert & Notification APIs

### 19. GET /api/alerts
**アラート一覧**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/alerts` |
| **Auth Required** | Yes (Admin) |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `severity` | string | all | アラート重要度 (critical, high, medium, low) |
| `status` | string | all | アラート状態 (active, resolved, acknowledged) |
| `limit` | number | 50 | 取得件数 |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "alerts": [
      {
        "id": "alert_123456",
        "severity": "high",
        "status": "active",
        "title": "High Response Time Detected",
        "description": "Average response time for POST /api/users exceeded 1000ms",
        "metric": "response_time",
        "current_value": 1450,
        "threshold": 1000,
        "layer": "presentation",
        "operation": "POST /api/users",
        "created_at": "2025-07-08T10:25:00Z",
        "last_triggered": "2025-07-08T10:29:45Z",
        "trigger_count": 5
      },
      {
        "id": "alert_123457",
        "severity": "medium",
        "status": "acknowledged",
        "title": "Memory Usage Above Normal",
        "description": "Memory usage exceeded 80MB threshold",
        "metric": "memory_usage",
        "current_value": 85,
        "threshold": 80,
        "layer": "infrastructure",
        "operation": "memory_management",
        "created_at": "2025-07-08T09:15:00Z",
        "acknowledged_at": "2025-07-08T09:20:00Z",
        "acknowledged_by": "admin@example.com"
      }
    ],
    "summary": {
      "total_alerts": 2,
      "active_count": 1,
      "critical_count": 0,
      "high_count": 1,
      "medium_count": 1,
      "low_count": 0
    }
  },
  "message": "Alerts retrieved successfully"
}
```

---

### 20. POST /api/alerts/{id}/acknowledge
**アラート確認**

| | |
|---|---|
| **Method** | POST |
| **Endpoint** | `/api/alerts/{id}/acknowledge` |
| **Auth Required** | Yes (Admin) |
| **Parameters** | `id`: Alert ID |

**Request Body:**
```json
{
  "comment": "Investigating the issue, temporary spike identified"
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "alert_id": "alert_123456",
    "status": "acknowledged",
    "acknowledged_at": "2025-07-08T10:35:00Z",
    "acknowledged_by": "admin@example.com",
    "comment": "Investigating the issue, temporary spike identified"
  },
  "message": "Alert acknowledged successfully"
}
```

---

## 📊 Export APIs

### 21. GET /api/export/metrics
**メトリクスデータエクスポート**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/export/metrics` |
| **Auth Required** | Yes (Admin) |

**Query Parameters:**
| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `format` | string | json | エクスポート形式 (json, csv, prometheus) |
| `period` | string | 24h | データ期間 |
| `layers` | string | all | 対象層 (カンマ区切り) |

**Response (200 OK - JSON format):**
```json
{
  "success": true,
  "data": {
    "export_info": {
      "format": "json",
      "period": "24h",
      "generated_at": "2025-07-08T10:30:00Z",
      "record_count": 15670
    },
    "metrics": [
      {
        "timestamp": "2025-07-08T10:29:00Z",
        "layer": "presentation",
        "operation": "POST /api/users",
        "duration_ms": 145,
        "status": "success",
        "user_id": "user_123",
        "request_id": "req_789"
      }
    ]
  },
  "message": "Metrics exported successfully"
}
```

**Response (200 OK - CSV format):**
```
Content-Type: text/csv
Content-Disposition: attachment; filename="metrics_20250708.csv"

timestamp,layer,operation,duration_ms,status,user_id,request_id
2025-07-08T10:29:00Z,presentation,POST /api/users,145,success,user_123,req_789
2025-07-08T10:28:55Z,application,CreateUserUseCase,89,success,user_123,req_789
```

---

## 🔧 Configuration APIs

### 22. GET /api/config/metrics
**メトリクス設定取得**

| | |
|---|---|
| **Method** | GET |
| **Endpoint** | `/api/config/metrics` |
| **Auth Required** | Yes (Admin) |

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "collection_settings": {
      "enabled": true,
      "collection_interval_seconds": 5,
      "retention_period_hours": 168,
      "max_records": 100000
    },
    "alert_thresholds": {
      "response_time_ms": 1000,
      "error_rate_percent": 5.0,
      "memory_usage_mb": 100,
      "concurrent_requests": 50
    },
    "dashboard_settings": {
      "auto_refresh_seconds": 5,
      "chart_data_points": 50,
      "real_time_enabled": true
    },
    "export_settings": {
      "allowed_formats": ["json", "csv", "prometheus"],
      "max_export_records": 50000,
      "compression_enabled": true
    }
  },
  "message": "Metrics configuration retrieved"
}
```

---

### 23. PUT /api/config/metrics
**メトリクス設定更新**

| | |
|---|---|
| **Method** | PUT |
| **Endpoint** | `/api/config/metrics` |
| **Auth Required** | Yes (Admin) |

**Request Body:**
```json
{
  "collection_settings": {
    "collection_interval_seconds": 10,
    "retention_period_hours": 72
  },
  "alert_thresholds": {
    "response_time_ms": 800,
    "error_rate_percent": 3.0
  }
}
```

**Response (200 OK):**
```json
{
  "success": true,
  "data": {
    "updated_settings": [
      "collection_interval_seconds",
      "retention_period_hours", 
      "response_time_threshold",
      "error_rate_threshold"
    ],
    "applied_at": "2025-07-08T10:35:00Z"
  },
  "message": "Metrics configuration updated successfully"
}
```

---

## 🎯 API Response Headers

すべてのAPIレスポンスには以下の共通ヘッダーが含まれます：

```
X-Request-ID: req_123456789          # リクエスト追跡ID
X-Response-Time: 145ms               # 処理時間
X-API-Version: v1                    # APIバージョン
X-Rate-Limit-Remaining: 4950         # 残りリクエスト数
X-Rate-Limit-Reset: 1625740800       # レート制限リセット時刻
```

## 🚫 Error Response Format

すべてのエラーレスポンスは統一された形式を使用します：

```json
{
  "success": false,
  "data": null,
  "error": {
    "code": "USER_NOT_FOUND",
    "message": "User with ID 123e4567-e89b-12d3-a456-426614174000 not found",
    "details": {
      "layer": "application",
      "operation": "get_user_usecase",
      "timestamp": "2025-07-08T10:30:00Z"
    }
  },
  "request_id": "req_123456",
  "processing_time_ms": 23
}
```

## 📋 HTTP Status Codes

| Status Code | Description | Usage |
|-------------|-------------|-------|
| 200 | OK | 成功（GET, PUT, DELETE） |
| 201 | Created | リソース作成成功（POST） |
| 204 | No Content | 成功（レスポンスボディなし） |
| 400 | Bad Request | バリデーションエラー |
| 401 | Unauthorized | 認証エラー |
| 403 | Forbidden | 認可エラー |
| 404 | Not Found | リソースが存在しない |
| 409 | Conflict | リソースの競合（重複など） |
| 422 | Unprocessable Entity | ビジネスルール違反 |
| 429 | Too Many Requests | レート制限超過 |
| 500 | Internal Server Error | サーバーエラー |
| 503 | Service Unavailable | サービス利用不可 |

## 🔒 Authentication & Authorization

### JWT Token Format
```
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
```

### Permission Levels
- **User**: 基本的なユーザー操作
- **Admin**: 全ユーザー管理 + メトリクス閲覧
- **SuperAdmin**: システム設定変更 + 設定管理

この完全なAPI仕様に基づいて、Self-Monitoring Clean Architectureアプリケーションを実装していきましょう！🚀📊🦀