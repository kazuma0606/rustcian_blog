# 🎨 Clean Architecture with In-Memory CQRS

## 🎯 Strategy: Focus on CA Beauty First

### Why In-Memory Implementation?
✅ **Focus on Architecture**: データベース接続エラーに時間を取られない  
✅ **CQRS分離が簡単**: Read/Write ストアを明確に分離可能  
✅ **高速開発**: 接続設定不要で即座に動作確認  
✅ **Pure CA**: アーキテクチャの美しさが際立つ  

## 🗄️ In-Memory CQRS Design

### Command Store (Write Side)
```rust
// src/infrastructure/repository/in_memory_user_command_repository.rs

use crate::domain::entity::user::User;
use crate::domain::repository::user_repository::UserCommandRepositoryInterface;
use crate::shared::error::{InfrastructureError, InfrastructureResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct InMemoryUserCommandRepository {
    // Write専用ストア - 正規化されたデータ
    store: Arc<RwLock<HashMap<Uuid, User>>>,
    email_index: Arc<RwLock<HashMap<String, Uuid>>>, // メール → ID のインデックス
}

impl InMemoryUserCommandRepository {
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
            email_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    // テスト用: 初期データ投入
    pub async fn seed_data(&self, users: Vec<User>) -> InfrastructureResult<()> {
        let mut store = self.store.write().await;
        let mut email_index = self.email_index.write().await;
        
        for user in users {
            email_index.insert(user.email().value().to_string(), user.id().clone());
            store.insert(user.id().clone(), user);
        }
        
        Ok(())
    }
}

#[async_trait]
impl UserCommandRepositoryInterface for InMemoryUserCommandRepository {
    async fn save(&self, user: &User) -> InfrastructureResult<()> {
        let mut store = self.store.write().await;
        let mut email_index = self.email_index.write().await;
        
        // メール重複チェック（既存ユーザーの更新時は除外）
        if let Some(existing_id) = email_index.get(user.email().value()) {
            if existing_id != user.id() {
                return Err(InfrastructureError::Database {
                    message: format!("Email already exists: {}", user.email().value()),
                });
            }
        }
        
        email_index.insert(user.email().value().to_string(), user.id().clone());
        store.insert(user.id().clone(), user.clone());
        
        // シミュレーション: Write側から Read側へのデータ同期
        println!("[CQRS] Command executed: User {} saved", user.id());
        
        Ok(())
    }
    
    async fn delete(&self, id: &Uuid) -> InfrastructureResult<()> {
        let mut store = self.store.write().await;
        let mut email_index = self.email_index.write().await;
        
        if let Some(user) = store.remove(id) {
            email_index.remove(user.email().value());
            println!("[CQRS] Command executed: User {} deleted", id);
            Ok(())
        } else {
            Err(InfrastructureError::Database {
                message: format!("User not found: {}", id),
            })
        }
    }
    
    async fn find_by_email_for_duplicate_check(&self, email: &str) -> InfrastructureResult<bool> {
        let email_index = self.email_index.read().await;
        Ok(email_index.contains_key(email))
    }
}
```

### Query Store (Read Side)
```rust
// src/infrastructure/repository/in_memory_user_query_repository.rs

use crate::application::dto::user::{GetUserResponseDto, ListUsersResponseDto};
use crate::domain::repository::user_repository::UserQueryRepositoryInterface;
use crate::shared::error::{InfrastructureError, InfrastructureResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserReadModel {
    pub id: String,
    pub email: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub search_terms: String, // 検索用の非正規化データ
}

pub struct InMemoryUserQueryRepository {
    // Read専用ストア - 非正規化された検索最適化データ
    read_models: Arc<RwLock<HashMap<Uuid, UserReadModel>>>,
    email_lookup: Arc<RwLock<HashMap<String, Uuid>>>,
    name_search_index: Arc<RwLock<HashMap<String, Vec<Uuid>>>>, // 名前検索用インデックス
}

impl InMemoryUserQueryRepository {
    pub fn new() -> Self {
        Self {
            read_models: Arc::new(RwLock::new(HashMap::new())),
            email_lookup: Arc::new(RwLock::new(HashMap::new())),
            name_search_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    // CQRS: Command側からのデータ同期（実際はイベント経由）
    pub async fn sync_from_command_side(&self, user: &crate::domain::entity::user::User) -> InfrastructureResult<()> {
        let read_model = UserReadModel {
            id: user.id().to_string(),
            email: user.email().value().to_string(),
            name: user.name().value().to_string(),
            created_at: user.created_at().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            updated_at: user.updated_at().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            search_terms: format!("{} {}", user.name().value(), user.email().value()).to_lowercase(),
        };
        
        let mut read_models = self.read_models.write().await;
        let mut email_lookup = self.email_lookup.write().await;
        let mut name_search_index = self.name_search_index.write().await;
        
        // インデックス更新
        email_lookup.insert(read_model.email.clone(), user.id().clone());
        
        // 名前検索インデックス更新
        let name_words: Vec<String> = read_model.name
            .split_whitespace()
            .map(|s| s.to_lowercase())
            .collect();
            
        for word in name_words {
            name_search_index
                .entry(word)
                .or_insert_with(Vec::new)
                .push(user.id().clone());
        }
        
        read_models.insert(user.id().clone(), read_model);
        
        println!("[CQRS] Read model updated for user: {}", user.id());
        Ok(())
    }
}

#[async_trait]
impl UserQueryRepositoryInterface for InMemoryUserQueryRepository {
    async fn find_by_id(&self, id: &Uuid) -> InfrastructureResult<Option<GetUserResponseDto>> {
        let read_models = self.read_models.read().await;
        
        if let Some(read_model) = read_models.get(id) {
            Ok(Some(GetUserResponseDto {
                id: read_model.id.clone(),
                email: read_model.email.clone(),
                name: read_model.name.clone(),
                created_at: read_model.created_at.clone(),
                updated_at: read_model.updated_at.clone(),
            }))
        } else {
            Ok(None)
        }
    }
    
    async fn find_by_email(&self, email: &str) -> InfrastructureResult<Option<GetUserResponseDto>> {
        let email_lookup = self.email_lookup.read().await;
        let read_models = self.read_models.read().await;
        
        if let Some(user_id) = email_lookup.get(email) {
            if let Some(read_model) = read_models.get(user_id) {
                return Ok(Some(GetUserResponseDto {
                    id: read_model.id.clone(),
                    email: read_model.email.clone(),
                    name: read_model.name.clone(),
                    created_at: read_model.created_at.clone(),
                    updated_at: read_model.updated_at.clone(),
                }));
            }
        }
        
        Ok(None)
    }
    
    async fn find_all(&self, limit: Option<i32>, offset: Option<i32>) -> InfrastructureResult<ListUsersResponseDto> {
        let read_models = self.read_models.read().await;
        
        let mut users: Vec<_> = read_models.values().collect();
        
        // 作成日時でソート（新しい順）
        users.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        
        // ページネーション
        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.unwrap_or(50) as usize;
        
        let total = users.len();
        let paginated_users: Vec<GetUserResponseDto> = users
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|read_model| GetUserResponseDto {
                id: read_model.id.clone(),
                email: read_model.email.clone(),
                name: read_model.name.clone(),
                created_at: read_model.created_at.clone(),
                updated_at: read_model.updated_at.clone(),
            })
            .collect();
        
        Ok(ListUsersResponseDto {
            users: paginated_users,
            total: total as i32,
            limit: limit as i32,
            offset: offset as i32,
        })
    }
    
    async fn search_by_name(&self, query: &str) -> InfrastructureResult<Vec<GetUserResponseDto>> {
        let name_search_index = self.name_search_index.read().await;
        let read_models = self.read_models.read().await;
        
        let search_term = query.to_lowercase();
        let mut matching_user_ids = Vec::new();
        
        // 名前インデックスから検索
        for (indexed_word, user_ids) in name_search_index.iter() {
            if indexed_word.contains(&search_term) {
                matching_user_ids.extend(user_ids);
            }
        }
        
        // 重複除去
        matching_user_ids.sort();
        matching_user_ids.dedup();
        
        let results: Vec<GetUserResponseDto> = matching_user_ids
            .into_iter()
            .filter_map(|user_id| read_models.get(&user_id))
            .map(|read_model| GetUserResponseDto {
                id: read_model.id.clone(),
                email: read_model.email.clone(),
                name: read_model.name.clone(),
                created_at: read_model.created_at.clone(),
                updated_at: read_model.updated_at.clone(),
            })
            .collect();
        
        Ok(results)
    }
}
```

## 🔄 CQRS Repository Interfaces

### Separated Command and Query Interfaces
```rust
// src/domain/repository/user_repository.rs

use crate::domain::entity::user::User;
use crate::application::dto::user::{GetUserResponseDto, ListUsersResponseDto};
use crate::shared::error::InfrastructureResult;
use async_trait::async_trait;
use uuid::Uuid;

// Command Side (Write Operations)
#[async_trait]
pub trait UserCommandRepositoryInterface: Send + Sync {
    async fn save(&self, user: &User) -> InfrastructureResult<()>;
    async fn delete(&self, id: &Uuid) -> InfrastructureResult<()>;
    async fn find_by_email_for_duplicate_check(&self, email: &str) -> InfrastructureResult<bool>;
}

// Query Side (Read Operations)
#[async_trait]
pub trait UserQueryRepositoryInterface: Send + Sync {
    async fn find_by_id(&self, id: &Uuid) -> InfrastructureResult<Option<GetUserResponseDto>>;
    async fn find_by_email(&self, email: &str) -> InfrastructureResult<Option<GetUserResponseDto>>;
    async fn find_all(&self, limit: Option<i32>, offset: Option<i32>) -> InfrastructureResult<ListUsersResponseDto>;
    async fn search_by_name(&self, query: &str) -> InfrastructureResult<Vec<GetUserResponseDto>>;
}
```

## 🎯 UseCase Implementation with CQRS

### Command UseCase
```rust
// src/application/usecase/user/create_user_usecase.rs

use crate::application::dto::user::{CreateUserRequestDto, CreateUserResponseDto};
use crate::domain::entity::user::User;
use crate::domain::repository::user_repository::UserCommandRepositoryInterface;
use crate::domain::service::id_generator::IdGeneratorInterface;
use crate::shared::error::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait CreateUserUsecaseInterface: Send + Sync {
    async fn execute(&self, request_dto: CreateUserRequestDto) -> ApplicationResult<CreateUserResponseDto>;
}

pub struct CreateUserUseCase<T, U>
where
    T: UserCommandRepositoryInterface + Send + Sync,
    U: IdGeneratorInterface + Send + Sync,
{
    command_repository: Arc<T>, // Command側のRepository
    id_generator: Arc<U>,
}

impl<T, U> CreateUserUseCase<T, U>
where
    T: UserCommandRepositoryInterface + Send + Sync,
    U: IdGeneratorInterface + Send + Sync,
{
    pub fn new(command_repository: Arc<T>, id_generator: Arc<U>) -> Self {
        Self {
            command_repository,
            id_generator,
        }
    }
}

#[async_trait]
impl<T, U> CreateUserUsecaseInterface for CreateUserUseCase<T, U>
where
    T: UserCommandRepositoryInterface + Send + Sync,
    U: IdGeneratorInterface + Send + Sync,
{
    async fn execute(&self, request_dto: CreateUserRequestDto) -> ApplicationResult<CreateUserResponseDto> {
        // 1. Business Logic Validation
        if self.command_repository
            .find_by_email_for_duplicate_check(&request_dto.email)
            .await
            .map_err(ApplicationError::Infrastructure)?
        {
            return Err(ApplicationError::EmailAlreadyExists {
                email: request_dto.email,
            });
        }
        
        // 2. Domain Entity Creation
        let user_id = self.id_generator.generate();
        let user = User::new(
            user_id,
            request_dto.email,
            request_dto.name,
            request_dto.password,
        ).map_err(ApplicationError::Domain)?;
        
        // 3. Command Execution (Write Side)
        self.command_repository
            .save(&user)
            .await
            .map_err(ApplicationError::Infrastructure)?;
        
        // 4. Return Response (from Domain Entity)
        Ok(CreateUserResponseDto::from(user))
    }
}
```

### Query UseCase
```rust
// src/application/usecase/user/get_user_usecase.rs

use crate::application::dto::user::{GetUserRequestDto, GetUserResponseDto};
use crate::domain::repository::user_repository::UserQueryRepositoryInterface;
use crate::shared::error::{ApplicationError, ApplicationResult};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait GetUserUsecaseInterface: Send + Sync {
    async fn execute(&self, request_dto: GetUserRequestDto) -> ApplicationResult<GetUserResponseDto>;
}

pub struct GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    query_repository: Arc<T>, // Query側のRepository
}

impl<T> GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    pub fn new(query_repository: Arc<T>) -> Self {
        Self { query_repository }
    }
}

#[async_trait]
impl<T> GetUserUsecaseInterface for GetUserUseCase<T>
where
    T: UserQueryRepositoryInterface + Send + Sync,
{
    async fn execute(&self, request_dto: GetUserRequestDto) -> ApplicationResult<GetUserResponseDto> {
        // Query Execution (Read Side)
        let user = self.query_repository
            .find_by_id(&request_dto.id)
            .await
            .map_err(ApplicationError::Infrastructure)?;
        
        match user {
            Some(user_dto) => Ok(user_dto),
            None => Err(ApplicationError::UserNotFound {
                id: request_dto.id.to_string(),
            }),
        }
    }
}
```

## 🔧 DI Setup with CQRS Separation

```rust
// src/main.rs

use crate::infrastructure::repository::{
    InMemoryUserCommandRepository,
    InMemoryUserQueryRepository,
};

async fn setup_dependencies() -> ApplicationResult<AppState> {
    // Infrastructure Layer - CQRS Repositories
    let command_repository = Arc::new(InMemoryUserCommandRepository::new());
    let query_repository = Arc::new(InMemoryUserQueryRepository::new());
    let id_generator = Arc::new(UuidGenerator::new());
    
    // テストデータ投入
    seed_test_data(&command_repository, &query_repository).await?;
    
    // Application Layer - Command UseCases
    let create_user_usecase = Arc::new(CreateUserUseCase::new(
        command_repository.clone(),
        id_generator.clone(),
    ));
    
    // Application Layer - Query UseCases  
    let get_user_usecase = Arc::new(GetUserUseCase::new(
        query_repository.clone(),
    ));
    
    let list_users_usecase = Arc::new(ListUsersUseCase::new(
        query_repository.clone(),
    ));
    
    Ok(AppState {
        // Command UseCases
        create_user_usecase,
        update_user_usecase,
        delete_user_usecase,
        
        // Query UseCases
        get_user_usecase,
        list_users_usecase,
        search_users_usecase,
    })
}

async fn seed_test_data(
    command_repo: &Arc<InMemoryUserCommandRepository>,
    query_repo: &Arc<InMemoryUserQueryRepository>,
) -> ApplicationResult<()> {
    let test_users = vec![
        User::new(
            Uuid::new_v4(),
            "alice@example.com".to_string(),
            "Alice Smith".to_string(),
            "password123".to_string(),
        )?,
        User::new(
            Uuid::new_v4(),
            "bob@example.com".to_string(),
            "Bob Johnson".to_string(),
            "password456".to_string(),
        )?,
    ];
    
    // Command側にデータ投入
    command_repo.seed_data(test_users.clone()).await?;
    
    // Query側に同期
    for user in test_users {
        query_repo.sync_from_command_side(&user).await?;
    }
    
    Ok(())
}
```

## ✅ Benefits of In-Memory CQRS

### 🎨 **Pure Clean Architecture Focus**
- データベース接続エラーなし
- アーキテクチャの美しさが際立つ
- ビジネスロジックに集中可能

### 🔄 **True CQRS Separation**
- Command/Query の責務が明確
- Read/Write の最適化が分かりやすい
- 将来のイベントソーシング対応も簡単

### ⚡ **High Development Speed**
- 即座に動作確認可能
- テストデータも簡単に投入
- CRUDの動作が目に見える

### 🧪 **Perfect for Testing**
- モックデータで完全テスト
- 状態のリセットが簡単
- 並行実行テストも可能

後でPostgreSQLに移行する時も、Repository実装を差し替えるだけで済みます！

まずは**この美しいCQRS構造**で完全なCAを構築しませんか？🎯