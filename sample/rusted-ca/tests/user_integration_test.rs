// tests/user_integration_test.rs
// ユーザー統合テスト（リポジトリ直接）
// 2025/7/8

use rusted_ca::domain::entity::user::User;
use rusted_ca::domain::repository::user_command_repository::UserCommandRepositoryInterface;
use rusted_ca::domain::repository::user_query_repository::UserQueryRepositoryInterface;
use rusted_ca::domain::value_object::{
    birth_date::BirthDate, email::Email, password::Password, phone::Phone, user_id::UserId,
    user_name::UserName,
};
use rusted_ca::infrastructure::di::container::DIContainer;
use std::sync::Arc;

#[tokio::test]
async fn test_user_repository_crud() {
    let di = DIContainer::new();
    let (command_repo, query_repo) = di.create_repositories().unwrap();

    // ユーザー作成
    let user_id = UserId::new("integration-test-id".to_string());
    let email = Email::new("integration@example.com".to_string()).unwrap();
    let name = UserName::new("Integration Test".to_string()).unwrap();
    let password = Password::new("password123".to_string()).unwrap();
    let phone = Some(Phone::new("09012345678".to_string()).unwrap());
    let birth_date = Some(BirthDate::new("2000-01-01".to_string()).unwrap());
    let user = User::new(
        user_id.clone(),
        email.clone(),
        name,
        password,
        phone,
        birth_date,
    )
    .unwrap();

    // save
    command_repo.save(&user).await.expect("ユーザー保存失敗");

    // find_by_id
    let found = query_repo.find_by_id(&user_id).await.expect("検索失敗");
    assert!(found.is_some(), "ユーザーが見つからない");
    let found = found.unwrap();
    assert_eq!(found.id.0, user_id.0);
    assert_eq!(found.email.0, email.0);

    // update
    let mut updated_user = found.clone();
    updated_user.name = UserName::new("Updated Name".to_string()).unwrap();
    command_repo
        .update(&updated_user)
        .await
        .expect("ユーザー更新失敗");
    let found2 = query_repo.find_by_id(&user_id).await.unwrap().unwrap();
    assert_eq!(found2.name.0, "Updated Name");

    // delete
    command_repo
        .delete(&user_id)
        .await
        .expect("ユーザー削除失敗");
    let deleted = query_repo.find_by_id(&user_id).await.unwrap();
    assert!(deleted.is_none(), "削除後もユーザーが存在する");
}
