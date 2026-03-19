//presentation/router/user_router.rs
// ユーザー関連ルーティング
// 2025/7/8

use crate::application::usecases::create_user_usecase::CreateUserUsecaseInterface;
use crate::application::usecases::delete_user_usecase::DeleteUserUsecaseInterface;
use crate::application::usecases::get_user_usecase::GetUserQueryUsecaseInterface;
use crate::application::usecases::update_user_usecase::UpdateUserUsecaseInterface;
use crate::presentation::controller::user_controller::UserController;
use crate::shared::middleware::auth_middleware::{AdminUser, AuthenticatedUser};
use axum::middleware::from_fn;
use axum::{
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

/// ユーザー関連のルーティング設定
///
/// 責務:
/// 1. Axumルーターの設定
/// 2. エンドポイントとControllerメソッドのマッピング
/// 3. クロージャでのController呼び出し
pub fn create_user_routes<T, U, V, W>(controller: Arc<UserController<T, U, V, W>>) -> Router
where
    T: CreateUserUsecaseInterface + Send + Sync + 'static,
    U: GetUserQueryUsecaseInterface + Send + Sync + 'static,
    V: UpdateUserUsecaseInterface + Send + Sync + 'static,
    W: DeleteUserUsecaseInterface + Send + Sync + 'static,
{
    Router::new()
        .route(
            "/users",
            post({
                let controller = controller.clone();
                move |auth: AuthenticatedUser, request| {
                    let controller = controller.clone();
                    async move { controller.create_user(auth, request).await }
                }
            }),
        )
        .route(
            "/users/:id",
            get({
                let controller = controller.clone();
                move |path| {
                    let controller = controller.clone();
                    async move { controller.get_user(path).await }
                }
            }),
        )
        .route(
            "/users/:id",
            put({
                let controller = controller.clone();
                move |auth: AuthenticatedUser, path, body| {
                    let controller = controller.clone();
                    async move { controller.update_user(auth, path, body).await }
                }
            }),
        )
        .route(
            "/users/:id",
            delete({
                let controller = controller.clone();
                move |auth: AuthenticatedUser, path| {
                    let controller = controller.clone();
                    async move { controller.delete_user(auth, path).await }
                }
            }),
        )
}
