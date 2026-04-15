//! Generic SeaORM repository with CRUD + paginated find.

use ddd_shared_kernel::pagination::{Page, PageRequest};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, Condition,
};

/// Thin wrapper around a [`DatabaseConnection`] exposing CRUD primitives for
/// an arbitrary SeaORM [`EntityTrait`].
pub struct BaseRepository<E: EntityTrait> {
    db: DatabaseConnection,
    _e: std::marker::PhantomData<E>,
}

impl<E: EntityTrait> BaseRepository<E> {
    /// Create a new repository.
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db, _e: std::marker::PhantomData }
    }

    /// Borrow the underlying connection.
    pub fn connection(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Look up a row by its primary key.
    pub async fn find_by_id<V>(&self, id: V) -> Result<Option<E::Model>, DbErr>
    where
        V: Into<<E::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>,
    {
        E::find_by_id(id).one(&self.db).await
    }

    /// Return every row.
    pub async fn find_all(&self) -> Result<Vec<E::Model>, DbErr> {
        E::find().all(&self.db).await
    }

    /// Insert a new row and return the resulting model.
    pub async fn save<A>(&self, model: A) -> Result<E::Model, DbErr>
    where
        A: ActiveModelTrait<Entity = E> + ActiveModelBehavior + Send + 'static,
        E::Model: IntoActiveModel<A>,
    {
        model.insert(&self.db).await
    }

    /// Update an existing row.
    pub async fn update<A>(&self, model: A) -> Result<E::Model, DbErr>
    where
        A: ActiveModelTrait<Entity = E> + ActiveModelBehavior + Send + 'static,
        E::Model: IntoActiveModel<A>,
    {
        model.update(&self.db).await
    }

    /// Delete a row by primary key and return the number of rows affected.
    pub async fn delete<V>(&self, id: V) -> Result<u64, DbErr>
    where
        V: Into<<E::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>,
    {
        let res = E::delete_by_id(id).exec(&self.db).await?;
        Ok(res.rows_affected)
    }

    /// Paginated query accepting a SeaORM [`Condition`] filter.
    pub async fn paginated_find(
        &self,
        filter: Condition,
        page_req: PageRequest,
    ) -> Result<Page<E::Model>, DbErr>
    where
        E::Model: Sync,
    {
        let page_size = u64::from(page_req.per_page());
        let paginator = E::find()
            .filter(filter)
            .paginate(&self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator
            .fetch_page(u64::from(page_req.page().saturating_sub(1)))
            .await?;

        Ok(Page::new(items, total, page_req.page(), page_req.per_page()))
    }
}
