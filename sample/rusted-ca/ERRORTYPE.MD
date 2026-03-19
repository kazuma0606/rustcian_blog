# 🎯 Layered Result and Error Types

## 📐 Error Type Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PresentationError                            │
│  HTTP関連エラー (400, 401, 403, 404, 500)                       │
│  PresentationResult<T> = Result<T, PresentationError>          │
├─────────────────────────────────────────────────────────────────┤
│                   ApplicationError                              │
│  ビジネスロジックエラー (EmailExists, UserNotFound...)          │
│  ApplicationResult<T> = Result<T, ApplicationError>            │
├─────────────────────────────────────────────────────────────────┤
│                     DomainError                                 │
│  ドメインルール違反 (InvalidEmail, InvalidPassword...)          │
│  DomainResult<T> = Result<T, DomainError>                      │
├─────────────────────────────────────────────────────────────────┤
│                 InfrastructureError                             │
│  外部システムエラー (Database, Network, ExternalService...)     │
│  InfrastructureResult<T> = Result<T, InfrastructureError>      │
└─────────────────────────────────────────────────────────────────┘
```

## 🏗️ Complete Error Type Definitions

### Domain Layer Errors
```rust
// src/shared/domain_error.rs

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainError {
    // Value Object Validation Errors
    #[error("Invalid email format: '{email}' - {reason}")]
    InvalidEmail { email: String, reason: String },
    
    #[error("Invalid user name: '{name}' - {reason}")]
    InvalidUserName { name: String, reason: String },
    
    #[error("Invalid password: {reason}")]
    InvalidPassword { reason: String },
    
    // Business Rule Violations
    #[error("Business rule violation: {rule} - {message}")]
    BusinessRuleViolation { rule: String, message: String },
    
    #[error("Entity validation failed: {entity} - {field}: {message}")]
    EntityValidationFailed { 
        entity: String, 
        field: String, 
        message: String 
    },
    
    // Domain Logic Errors
    #[error("Invariant violation: {message}")]
    InvariantViolation { message: String },
}

// Domain Layer Result Type
pub type DomainResult<T> = Result<T, DomainError>;
```

### Application Layer Errors
```rust
// src/shared/application_error.rs

use thiserror::Error;
use crate::domain::error::DomainError;
use crate::infrastructure::error::InfrastructureError;

#[derive(Error, Debug)]
pub enum ApplicationError {
    // UseCase Specific Errors
    #[error("User not found: {id}")]
    UserNotFound { id: String },
    
    #[error("Email already exists: {email}")]
    EmailAlreadyExists { email: String },
    
    #[error("Authorization failed: {message}")]
    AuthorizationFailed { message: String },
    
    #[error("Operation not permitted: {operation} - {reason}")]
    OperationNotPermitted { operation: String, reason: String },
    
    // Input Validation Errors
    #[error("Validation failed: {field} - {message}")]
    ValidationFailed { field: String, message: String },
    
    #[error("Invalid input: {input} - {reason}")]
    InvalidInput { input: String, reason: String },
    
    // UseCase Flow Errors
    #[error("Precondition failed: {condition}")]
    PreconditionFailed { condition: String },
    
    #[error("Postcondition failed: {condition}")]
    PostconditionFailed { condition: String },
    
    // Dependency Errors (auto-conversion from lower layers)
    #[error("Domain error: {0}")]
    Domain(#[from] DomainError),
    
    #[error("Infrastructure error: {0}")]
    Infrastructure(#[from] InfrastructureError),
}

// Application Layer Result Type
pub type ApplicationResult<T> = Result<T, ApplicationError>;
```

### Infrastructure Layer Errors
```rust
// src/shared/infrastructure_error.rs

use thiserror::Error;

#[derive(Error, Debug)]
pub enum InfrastructureError {
    // Database Errors
    #[error("Database connection failed: {message}")]
    DatabaseConnection { message: String },
    
    #[error("Database query failed: {query} - {message}")]
    DatabaseQuery { query: String, message: String },
    
    #[error("Database transaction failed: {message}")]
    DatabaseTransaction { message: String },
    
    #[error("Data serialization failed: {data_type} - {message}")]
    DataSerialization { data_type: String, message: String },
    
    // Network/External Service Errors
    #[error("Network error: {endpoint} - {message}")]
    Network { endpoint: String, message: String },
    
    #[error("External service error: {service} - {status}: {message}")]
    ExternalService { 
        service: String, 
        status: String, 
        message: String 
    },
    
    #[error("API timeout: {service} - {timeout_ms}ms")]
    Timeout { service: String, timeout_ms: u64 },
    
    // Configuration Errors
    #[error("Configuration error: {key} - {message}")]
    Configuration { key: String, message: String },
    
    #[error("Environment variable missing: {var_name}")]
    EnvironmentVariable { var_name: String },
    
    // File/IO Errors
    #[error("File operation failed: {path} - {message}")]
    FileOperation { path: String, message: String },
    
    #[error("Resource not available: {resource} - {message}")]
    ResourceUnavailable { resource: String, message: String },
}

// Infrastructure Layer Result Type
pub type InfrastructureResult<T> = Result<T, InfrastructureError>;
```

### Presentation Layer Errors
```rust
// src/shared/presentation_error.rs

use thiserror::Error;
use crate::application::error::ApplicationError;
use axum::http::StatusCode;

#[derive(Error, Debug)]
pub enum PresentationError {
    // HTTP Request Errors
    #[error("Bad request: {message}")]
    BadRequest { message: String },
    
    #[error("Unauthorized: {message}")]
    Unauthorized { message: String },
    
    #[error("Forbidden: {message}")]
    Forbidden { message: String },
    
    #[error("Not found: {resource}")]
    NotFound { resource: String },
    
    #[error("Method not allowed: {method} on {path}")]
    MethodNotAllowed { method: String, path: String },
    
    #[error("Request timeout: {message}")]
    RequestTimeout { message: String },
    
    #[error("Payload too large: {size} bytes (max: {max} bytes)")]
    PayloadTooLarge { size: usize, max: usize },
    
    // Content Type Errors
    #[error("Unsupported media type: {media_type}")]
    UnsupportedMediaType { media_type: String },
    
    #[error("Invalid content type: expected {expected}, got {actual}")]
    InvalidContentType { expected: String, actual: String },
    
    // Serialization Errors
    #[error("JSON serialization failed: {message}")]
    JsonSerialization { message: String },
    
    #[error("JSON deserialization failed: {message}")]
    JsonDeserialization { message: String },
    
    // Server Errors
    #[error("Internal server error: {message}")]
    InternalServer { message: String },
    
    #[error("Service unavailable: {message}")]
    ServiceUnavailable { message: String },
    
    // Dependency Error (auto-conversion from Application layer)
    #[error("Application error: {0}")]
    Application(#[from] ApplicationError),
}

// Presentation Layer Result Type
pub type PresentationResult<T> = Result<T, PresentationError>;

// HTTP Status Code Mapping
impl PresentationError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            PresentationError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            PresentationError::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            PresentationError::Forbidden { .. } => StatusCode::FORBIDDEN,
            PresentationError::NotFound { .. } => StatusCode::NOT_FOUND,
            PresentationError::MethodNotAllowed { .. } => StatusCode::METHOD_NOT_ALLOWED,
            PresentationError::RequestTimeout { .. } => StatusCode::REQUEST_TIMEOUT,
            PresentationError::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            PresentationError::UnsupportedMediaType { .. } => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            PresentationError::InvalidContentType { .. } => StatusCode::BAD_REQUEST,
            PresentationError::JsonSerialization { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PresentationError::JsonDeserialization { .. } => StatusCode::BAD_REQUEST,
            PresentationError::InternalServer { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            PresentationError::ServiceUnavailable { .. } => StatusCode::SERVICE_UNAVAILABLE,
            
            // Application layer error mapping
            PresentationError::Application(app_error) => match app_error {
                ApplicationError::UserNotFound { .. } => StatusCode::NOT_FOUND,
                ApplicationError::EmailAlreadyExists { .. } => StatusCode::CONFLICT,
                ApplicationError::AuthorizationFailed { .. } => StatusCode::FORBIDDEN,
                ApplicationError::ValidationFailed { .. } => StatusCode::BAD_REQUEST,
                ApplicationError::InvalidInput { .. } => StatusCode::BAD_REQUEST,
                ApplicationError::PreconditionFailed { .. } => StatusCode::PRECONDITION_FAILED,
                ApplicationError::OperationNotPermitted { .. } => StatusCode::FORBIDDEN,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}
```

## 🔄 Error Conversion Flow

### Automatic Error Propagation
```rust
// Domain → Application (automatic via #[from])
let user = User::new(email, name, password)?; // DomainError → ApplicationError

// Infrastructure → Application (automatic via #[from])
let saved_user = repository.save(&user).await?; // InfrastructureError → ApplicationError

// Application → Presentation (automatic via #[from])
let result = usecase.execute(request).await?; // ApplicationError → PresentationError
```

### Manual Error Conversion (when needed)
```rust
// Infrastructure-specific error to Application
let users = repository.find_all().await
    .map_err(|e| ApplicationError::Infrastructure(e))?;

// Application-specific error to Presentation
let user = usecase.get_user(id).await
    .map_err(|e| match e {
        ApplicationError::UserNotFound { .. } => PresentationError::NotFound {
            resource: "User".to_string(),
        },
        _ => PresentationError::Application(e),
    })?;
```

## 🎯 Layer-Specific Usage Examples

### Domain Layer Usage
```rust
// src/domain/value_object/email.rs

use crate::domain::error::{DomainError, DomainResult};

impl Email {
    pub fn new(value: String) -> DomainResult<Self> {
        if value.is_empty() {
            return Err(DomainError::InvalidEmail {
                email: value,
                reason: "Email cannot be empty".to_string(),
            });
        }
        
        if !Self::is_valid_format(&value) {
            return Err(DomainError::InvalidEmail {
                email: value,
                reason: "Invalid email format".to_string(),
            });
        }
        
        Ok(Self(value))
    }
}
```

### Application Layer Usage
```rust
// src/application/usecase/user/create_user_usecase.rs

use crate::application::error::{ApplicationError, ApplicationResult};

impl CreateUserUseCase {
    pub async fn execute(&self, request: CreateUserRequestDto) -> ApplicationResult<CreateUserResponseDto> {
        // Validation
        if self.repository.find_by_email(&request.email).await?.is_some() {
            return Err(ApplicationError::EmailAlreadyExists {
                email: request.email,
            });
        }
        
        // Domain object creation (DomainError auto-converts to ApplicationError)
        let user = User::new(id, request.email, request.name, request.password)?;
        
        // Infrastructure call (InfrastructureError auto-converts to ApplicationError)
        self.repository.save(&user).await?;
        
        Ok(CreateUserResponseDto::from(user))
    }
}
```

### Infrastructure Layer Usage
```rust
// src/infrastructure/repository/in_memory_user_repository.rs

use crate::infrastructure::error::{InfrastructureError, InfrastructureResult};

impl UserCommandRepositoryInterface for InMemoryUserRepository {
    async fn save(&self, user: &User) -> InfrastructureResult<()> {
        let mut store = self.store.write().await;
        
        // Simulate potential infrastructure errors
        if store.len() >= 1000 {
            return Err(InfrastructureError::ResourceUnavailable {
                resource: "User storage".to_string(),
                message: "Storage limit reached".to_string(),
            });
        }
        
        store.insert(user.id().clone(), user.clone());
        Ok(())
    }
}
```

### Presentation Layer Usage
```rust
// src/presentation/controller/user_controller.rs

use crate::presentation::error::{PresentationError, PresentationResult};

pub async fn create_user(
    State(usecase): State<Arc<dyn CreateUserUsecaseInterface>>,
    Json(request): Json<CreateUserRequest>,
) -> PresentationResult<Json<ApiResponse<CreateUserResponse>>> {
    // Input validation
    if request.email.is_empty() {
        return Err(PresentationError::BadRequest {
            message: "Email is required".to_string(),
        });
    }
    
    // UseCase execution (ApplicationError auto-converts to PresentationError)
    let dto = CreateUserRequestDto::from(request);
    let result = usecase.execute(dto).await?;
    
    Ok(Json(ApiResponse::success(
        CreateUserResponse::from(result),
        "User created successfully".to_string(),
    )))
}
```

## ✅ Benefits of Layered Error Types

### 🎯 **Clear Error Source**
```rust
DomainResult<User>          // ドメインルール違反
ApplicationResult<UserDto>  // ビジネスロジックエラー
InfrastructureResult<()>    // 外部システムエラー
PresentationResult<Json>    // HTTP関連エラー
```

### 🔄 **Automatic Error Conversion**
```rust
// #[from] による自動変換
DomainError → ApplicationError → PresentationError
InfrastructureError → ApplicationError → PresentationError
```

### 🧪 **Layer-Specific Testing**
```rust
// Domain layer test
assert_eq!(Email::new(""), Err(DomainError::InvalidEmail { ... }));

// Application layer test  
assert_eq!(usecase.execute(cmd), Err(ApplicationError::EmailAlreadyExists { ... }));

// Presentation layer test
assert_eq!(response.status(), StatusCode::CONFLICT);
```

### 📊 **Error Analytics**
```rust
// エラーの発生層が明確
match error {
    PresentationError::Application(ApplicationError::Domain(DomainError::InvalidEmail { .. })) => {
        // Domain layer でのバリデーションエラー
    },
    PresentationError::Application(ApplicationError::Infrastructure(InfrastructureError::Database { .. })) => {
        // Infrastructure layer でのDBエラー
    },
}
```