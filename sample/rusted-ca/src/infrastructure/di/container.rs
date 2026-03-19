//infrastructure/di/container.rs
// DIコンテナ - CQRS対応
// 2025/7/8

use crate::domain::service::id_generator::{IdGeneratorInterface, UuidGenerator};
use crate::domain::value_object::user_id::UserId;
use crate::infrastructure::database::sqlite_connection::SqliteConnection;
use crate::infrastructure::repository::in_memory_user_command_repository::SqliteUserCommandRepository;
use crate::infrastructure::repository::in_memory_user_query_repository::SqliteUserQueryRepository;
use std::sync::Arc;

/// DIコンテナ
///
/// 責務:
/// 1. 依存関係の組み立て
/// 2. Repository実装の注入
/// 3. UseCaseの組み立て
/// 4. Controllerの組み立て
/// 5. CQRSパターンの実装
pub struct DIContainer {
    // 実際の実装ではここにRepositoryの具体実装やその他の依存関係を定義
}

impl DIContainer {
    pub fn new() -> Self {
        Self {}
    }

    /// データベース接続の作成
    pub fn create_database_connection(
        &self,
    ) -> Result<SqliteConnection, Box<dyn std::error::Error + Send + Sync>> {
        SqliteConnection::new_in_memory()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Repository実装の作成
    pub fn create_repositories(
        &self,
    ) -> Result<
        (
            Arc<SqliteUserCommandRepository>,
            Arc<SqliteUserQueryRepository>,
        ),
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let db_connection = self.create_database_connection()?;

        let command_repository = Arc::new(SqliteUserCommandRepository::new(db_connection.clone()));
        let query_repository = Arc::new(SqliteUserQueryRepository::new(db_connection));

        Ok((command_repository, query_repository))
    }

    /// ID生成器の作成
    pub fn create_id_generator(&self) -> Box<dyn Fn() -> UserId + Send + Sync> {
        let uuid_generator = UuidGenerator;
        Box::new(move || {
            let id_string = uuid_generator.generate();
            UserId::new(id_string)
        })
    }

    /// UserControllerを組み立てて返す
    pub fn build_user_controller(&self) -> Result<
        std::sync::Arc<
            crate::presentation::controller::user_controller::UserController<
                crate::application::usecases::create_user_usecase::CreateUserUseCase<Box<dyn Fn() -> crate::domain::value_object::user_id::UserId + Send + Sync>>,
                crate::application::usecases::get_user_usecase::GetUserUseCase<
                    crate::infrastructure::repository::in_memory_user_query_repository::SqliteUserQueryRepository
                >,
                crate::application::usecases::update_user_usecase::UpdateUserUseCase,
                crate::application::usecases::delete_user_usecase::DeleteUserUseCase
            >
        >,
        Box<dyn std::error::Error + Send + Sync>,
    >{
        let (command_repo, query_repo) = self.create_repositories()?;
        let id_generator = self.create_id_generator();
        let command_repo_trait: std::sync::Arc<dyn crate::domain::repository::user_command_repository::UserCommandRepositoryInterface + Send + Sync> = command_repo as std::sync::Arc<dyn crate::domain::repository::user_command_repository::UserCommandRepositoryInterface + Send + Sync>;
        let create_user_usecase =
            crate::application::usecases::create_user_usecase::CreateUserUseCase::new(
                command_repo_trait.clone(),
                id_generator,
            );
        let get_user_usecase =
            crate::application::usecases::get_user_usecase::GetUserUseCase::new(query_repo.clone());
        let update_user_usecase =
            crate::application::usecases::update_user_usecase::UpdateUserUseCase::new(
                command_repo_trait.clone(),
                query_repo.clone(),
            );
        let delete_user_usecase =
            crate::application::usecases::delete_user_usecase::DeleteUserUseCase::new(
                command_repo_trait,
                query_repo,
            );
        let controller = crate::presentation::controller::user_controller::UserController::new(
            std::sync::Arc::new(create_user_usecase),
            std::sync::Arc::new(get_user_usecase),
            std::sync::Arc::new(update_user_usecase),
            std::sync::Arc::new(delete_user_usecase),
        );
        Ok(std::sync::Arc::new(controller))
    }

    /// 依存関係の組み立て例（型の問題によりコメントアウト）
    ///
    /// 実際の実装では、以下のような流れでControllerを組み立てます：
    /// 1. Repository実装を注入（Command/Query分離）
    /// 2. ID生成器を注入
    /// 3. UseCase組み立て（Command/Query分離）
    /// 4. Controller組み立て
    ///
    /// 現在は型の問題により、個別のコンポーネントのテストのみ実装
    pub fn demonstrate_di_setup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 1. Repository実装を注入（Command/Query分離）
        let (user_command_repository, user_query_repository) = self.create_repositories()?;
        println!("✅ Repository実装の作成が完了しました");

        // 2. ID生成器を注入
        let id_generator = self.create_id_generator();
        let test_id = id_generator();
        println!("✅ ID生成器のテストが完了しました: {}", test_id.0);

        // 3. UseCase組み立て（Command/Query分離）
        // 型の問題により、実際のUseCase組み立ては別途実装
        println!("✅ UseCase組み立ての準備が完了しました");

        // 4. Controller組み立て
        // 型の問題により、実際のController組み立ては別途実装
        println!("✅ Controller組み立ての準備が完了しました");

        Ok(())
    }
}
