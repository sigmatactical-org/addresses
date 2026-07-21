pub use sigma_pg::api::StoreError;

use sqlx::{PgPool, Row};

use crate::model::{Address, AddressCategory, CreateAddress, UpdateAddress};

/// Entity name used in [`StoreError::NotFound`] messages.
const ENTITY: &str = "address";

#[derive(Debug, Clone)]
pub struct AddressStore {
    pool: PgPool,
}

impl AddressStore {
    pub async fn connect() -> Result<Self, StoreError> {
        let pool = sigma_pg::connect_as("addresses").await?;
        Ok(Self { pool })
    }

    #[cfg(test)]
    pub async fn connect_empty() -> Result<Self, StoreError> {
        let store = Self::connect().await?;
        sigma_pg::assert_disposable_test_db(&store.pool).await;
        sqlx::query("TRUNCATE addresses.addresses")
            .execute(&store.pool)
            .await?;
        Ok(store)
    }

    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// List `user_id`'s addresses, optionally restricted to one `category`
    /// (filtered in SQL, not in Rust, so a category listing never reads the
    /// user's other-category rows). Every read in this service is scoped by
    /// the caller's verified session `user_id` — there is no "list all
    /// addresses" endpoint.
    pub async fn list_for_user(
        &self,
        user_id: &str,
        category: Option<AddressCategory>,
    ) -> Result<Vec<Address>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, user_id, category, label, recipient_name, line1, line2, city, region, \
             postal_code, country, is_default, updated_at \
             FROM addresses.addresses \
             WHERE user_id = $1 AND ($2::text IS NULL OR category = $2) \
             ORDER BY category, updated_at DESC",
        )
        .bind(user_id)
        .bind(category.map(AddressCategory::as_str))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_address).collect()
    }

    /// Fetch one address, scoped to `user_id`. Returns [`StoreError::NotFound`]
    /// both when the id doesn't exist and when it belongs to a different
    /// user — the two cases are indistinguishable to the caller so a user
    /// can't probe for the existence of another user's address ids.
    pub async fn get_for_user(&self, user_id: &str, id: &str) -> Result<Address, StoreError> {
        let row = sqlx::query(
            "SELECT id, user_id, category, label, recipient_name, line1, line2, city, region, \
             postal_code, country, is_default, updated_at \
             FROM addresses.addresses WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?;
        match row {
            Some(row) => row_to_address(row),
            None => Err(StoreError::NotFound(ENTITY)),
        }
    }

    /// Fetch one address by id, unscoped by user. Used only by the internal
    /// JSON API so callers such as payments can validate that a
    /// `billing_address_id` belongs to the expected user by comparing
    /// `user_id` themselves.
    pub async fn get_any(&self, id: &str) -> Result<Option<Address>, StoreError> {
        let row = sqlx::query(
            "SELECT id, user_id, category, label, recipient_name, line1, line2, city, region, \
             postal_code, country, is_default, updated_at \
             FROM addresses.addresses WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_address).transpose()
    }

    pub async fn create(&self, user_id: &str, input: CreateAddress) -> Result<Address, StoreError> {
        validate_fields(
            &input.line1,
            &input.city,
            &input.postal_code,
            &input.country,
        )?;
        let address = Address::new(user_id, input);
        sqlx::query(
            "INSERT INTO addresses.addresses \
             (id, user_id, category, label, recipient_name, line1, line2, city, region, \
              postal_code, country, is_default, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(&address.id)
        .bind(&address.user_id)
        .bind(address.category.as_str())
        .bind(&address.label)
        .bind(&address.recipient_name)
        .bind(&address.line1)
        .bind(&address.line2)
        .bind(&address.city)
        .bind(&address.region)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(address.is_default)
        .bind(address.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(address)
    }

    pub async fn update(
        &self,
        user_id: &str,
        id: &str,
        input: UpdateAddress,
    ) -> Result<Address, StoreError> {
        validate_fields(
            &input.line1,
            &input.city,
            &input.postal_code,
            &input.country,
        )?;
        let mut address = self.get_for_user(user_id, id).await?;
        address.apply_update(input);
        sqlx::query(
            "UPDATE addresses.addresses SET label = $3, recipient_name = $4, line1 = $5, \
             line2 = $6, city = $7, region = $8, postal_code = $9, country = $10, updated_at = $11 \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(&address.id)
        .bind(user_id)
        .bind(&address.label)
        .bind(&address.recipient_name)
        .bind(&address.line1)
        .bind(&address.line2)
        .bind(&address.city)
        .bind(&address.region)
        .bind(&address.postal_code)
        .bind(&address.country)
        .bind(address.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(address)
    }

    pub async fn delete(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let result = sqlx::query("DELETE FROM addresses.addresses WHERE id = $1 AND user_id = $2")
            .bind(id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::NotFound(ENTITY));
        }
        Ok(())
    }

    /// Promote `id` to the default address for its `(user_id, category)`.
    /// The DB enforces at most one default per user/category via a partial
    /// unique index, so any other default in the same category must be
    /// cleared in the same transaction before this row is set, or the second
    /// write would violate the index.
    pub async fn set_default(&self, user_id: &str, id: &str) -> Result<(), StoreError> {
        let address = self.get_for_user(user_id, id).await?;
        let mut tx = self.pool.begin().await?;
        sqlx::query(
            "UPDATE addresses.addresses SET is_default = false \
             WHERE user_id = $1 AND category = $2 AND id != $3",
        )
        .bind(user_id)
        .bind(address.category.as_str())
        .bind(id)
        .execute(&mut *tx)
        .await?;
        let result = sqlx::query(
            "UPDATE addresses.addresses SET is_default = true, updated_at = now() \
             WHERE id = $1 AND user_id = $2",
        )
        .bind(id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
        if result.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(StoreError::NotFound(ENTITY));
        }
        tx.commit().await?;
        Ok(())
    }
}

fn validate_fields(
    line1: &str,
    city: &str,
    postal_code: &str,
    country: &str,
) -> Result<(), StoreError> {
    let required = [
        ("line1", line1),
        ("city", city),
        ("postal_code", postal_code),
        ("country", country),
    ];
    for (name, value) in required {
        if value.trim().is_empty() {
            return Err(StoreError::InvalidInput(format!("{name} is required")));
        }
    }
    Ok(())
}

fn row_to_address(row: sqlx::postgres::PgRow) -> Result<Address, StoreError> {
    let category_str: String = row.get("category");
    Ok(Address {
        id: row.get("id"),
        user_id: row.get("user_id"),
        category: AddressCategory::parse(&category_str).map_err(StoreError::InvalidInput)?,
        label: row.get("label"),
        recipient_name: row.get("recipient_name"),
        line1: row.get("line1"),
        line2: row.get("line2"),
        city: row.get("city"),
        region: row.get("region"),
        postal_code: row.get("postal_code"),
        country: row.get("country"),
        is_default: row.get("is_default"),
        updated_at: row.get("updated_at"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_store() -> AddressStore {
        AddressStore::connect_empty()
            .await
            .expect("PostgreSQL required for tests")
    }

    fn shipping_input(line1: &str) -> CreateAddress {
        CreateAddress {
            category: AddressCategory::Shipping,
            label: Some("Home".to_string()),
            recipient_name: Some("Jane Doe".to_string()),
            line1: line1.to_string(),
            line2: None,
            city: "Springfield".to_string(),
            region: Some("IL".to_string()),
            postal_code: "62704".to_string(),
            country: "US".to_string(),
        }
    }

    fn billing_input(line1: &str) -> CreateAddress {
        CreateAddress {
            category: AddressCategory::Billing,
            ..shipping_input(line1)
        }
    }

    #[tokio::test]
    async fn create_list_update_delete_round_trip() {
        let store = test_store().await;
        let address = store
            .create("user-1", shipping_input("100 Main St"))
            .await
            .unwrap();
        assert_eq!(address.user_id, "user-1");
        assert!(!address.is_default);

        let listed = store.list_for_user("user-1", None).await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, address.id);

        let updated = store
            .update(
                "user-1",
                &address.id,
                UpdateAddress {
                    label: Some("Office".to_string()),
                    recipient_name: None,
                    line1: "200 Main St".to_string(),
                    line2: None,
                    city: "Springfield".to_string(),
                    region: Some("IL".to_string()),
                    postal_code: "62704".to_string(),
                    country: "US".to_string(),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.line1, "200 Main St");
        assert_eq!(updated.label.as_deref(), Some("Office"));

        store.delete("user-1", &address.id).await.unwrap();
        let err = store.get_for_user("user-1", &address.id).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn list_for_user_filters_by_category_in_sql() {
        let store = test_store().await;
        let billing = store
            .create("user-1", billing_input("1 Billing Way"))
            .await
            .unwrap();
        let shipping = store
            .create("user-1", shipping_input("1 Shipping Way"))
            .await
            .unwrap();

        let billing_only = store
            .list_for_user("user-1", Some(AddressCategory::Billing))
            .await
            .unwrap();
        assert_eq!(billing_only.len(), 1);
        assert_eq!(billing_only[0].id, billing.id);

        let shipping_only = store
            .list_for_user("user-1", Some(AddressCategory::Shipping))
            .await
            .unwrap();
        assert_eq!(shipping_only.len(), 1);
        assert_eq!(shipping_only[0].id, shipping.id);

        assert_eq!(store.list_for_user("user-1", None).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn get_for_user_hides_other_users_addresses() {
        let store = test_store().await;
        let address = store
            .create("user-1", shipping_input("100 Main St"))
            .await
            .unwrap();
        let err = store.get_for_user("user-2", &address.id).await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn delete_missing_address_returns_not_found() {
        let store = test_store().await;
        let err = store.delete("user-1", "does-not-exist").await.unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn create_rejects_missing_required_field() {
        let store = test_store().await;
        let err = store
            .create("user-1", shipping_input(""))
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::InvalidInput(_)));
    }

    #[tokio::test]
    async fn set_default_clears_previous_default_in_same_category() {
        let store = test_store().await;
        let a = store
            .create("user-1", shipping_input("100 Main St"))
            .await
            .unwrap();
        let b = store
            .create("user-1", shipping_input("200 Main St"))
            .await
            .unwrap();

        store.set_default("user-1", &a.id).await.unwrap();
        let listed = store.list_for_user("user-1", None).await.unwrap();
        assert!(listed.iter().find(|x| x.id == a.id).unwrap().is_default);
        assert!(!listed.iter().find(|x| x.id == b.id).unwrap().is_default);

        store.set_default("user-1", &b.id).await.unwrap();
        let listed = store.list_for_user("user-1", None).await.unwrap();
        assert!(!listed.iter().find(|x| x.id == a.id).unwrap().is_default);
        assert!(listed.iter().find(|x| x.id == b.id).unwrap().is_default);
    }

    #[tokio::test]
    async fn default_is_scoped_per_category_not_globally() {
        let store = test_store().await;
        let billing = store
            .create("user-1", billing_input("1 Billing Way"))
            .await
            .unwrap();
        let shipping = store
            .create("user-1", shipping_input("1 Shipping Way"))
            .await
            .unwrap();

        // Setting a default billing address and a default shipping address
        // for the same user must both succeed — the partial unique index is
        // per (user_id, category), not per user.
        store.set_default("user-1", &billing.id).await.unwrap();
        store.set_default("user-1", &shipping.id).await.unwrap();

        let listed = store.list_for_user("user-1", None).await.unwrap();
        assert!(
            listed
                .iter()
                .find(|x| x.id == billing.id)
                .unwrap()
                .is_default
        );
        assert!(
            listed
                .iter()
                .find(|x| x.id == shipping.id)
                .unwrap()
                .is_default
        );
    }

    #[tokio::test]
    async fn set_default_missing_address_returns_not_found() {
        let store = test_store().await;
        let err = store
            .set_default("user-1", "does-not-exist")
            .await
            .unwrap_err();
        assert!(matches!(err, StoreError::NotFound(_)));
    }

    #[tokio::test]
    async fn get_any_is_unscoped_by_user() {
        let store = test_store().await;
        let address = store
            .create("user-1", shipping_input("100 Main St"))
            .await
            .unwrap();
        let found = store.get_any(&address.id).await.unwrap();
        assert_eq!(found.unwrap().user_id, "user-1");
        assert!(store.get_any("does-not-exist").await.unwrap().is_none());
    }
}
