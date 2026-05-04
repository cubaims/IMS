use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Row};

use cuba_shared::{AppError, AppResult};

use crate::application::{
    BomRepository, CreateBomComponentCommand, CreateBomHeaderCommand, CreateCustomerCommand,
    CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
    CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
    CreateSupplierCommand, CreateWorkCenterCommand, CustomerRepository, MasterDataQuery,
    MaterialRepository, MaterialSupplierRepository, ProductVariantRepository,
    QualityMasterRepository, StorageBinRepository, SupplierRepository, UpdateBomComponentCommand,
    UpdateBomHeaderCommand, UpdateCustomerCommand, UpdateDefectCodeCommand,
    UpdateInspectionCharCommand, UpdateMaterialCommand, UpdateMaterialSupplierCommand,
    UpdateProductVariantCommand, UpdateStorageBinCommand, UpdateSupplierCommand,
    UpdateWorkCenterCommand, WorkCenterRepository,
};

#[derive(Clone)]
pub struct PostgresMasterDataRepository {
    pool: PgPool,
}

impl PostgresMasterDataRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(dead_code)]
    async fn fetch_json(&self, sql: &str) -> AppResult<Value> {
        let row = sqlx::query(sql).fetch_one(&self.pool).await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn fetch_json_with_id(&self, sql: &str, id: &str) -> AppResult<Value> {
        let row = sqlx::query(sql).bind(id).fetch_optional(&self.pool).await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("record not found: {id}")));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn fetch_list(&self, base_sql: &str, query: MasterDataQuery) -> AppResult<Value> {
        let sql = format!(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
            FROM (
                {base_sql}
                LIMIT $3 OFFSET $4
            ) t
            "#
        );

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;

        let row = sqlx::query(&sql)
            .bind(page as i32)
            .bind(page_size as i32)
            .bind(limit)
            .bind(offset)
            .fetch_one(&self.pool)
            .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn affected_to_json(
        &self,
        result: sqlx::postgres::PgQueryResult,
        id_name: &str,
        id_value: &str,
    ) -> AppResult<Value> {
        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "record not found: {id_name}={id_value}"
            )));
        }

        Ok(serde_json::json!({
            id_name: id_value,
            "affected": result.rows_affected()
        }))
    }
}

#[async_trait]
impl MaterialRepository for PostgresMasterDataRepository {
    async fn list_materials(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                material_id,
                material_name,
                material_type::text AS material_type,
                base_unit,
                default_zone,
                safety_stock,
                reorder_point,
                standard_price,
                map_price,
                current_stock,
                status,
                created_at,
                updated_at
            FROM mdm.mdm_materials
            ORDER BY material_id
            "#,
            query,
        )
        .await
    }

    async fn get_material(&self, material_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    material_id,
                    material_name,
                    material_type::text AS material_type,
                    base_unit,
                    default_zone,
                    safety_stock,
                    reorder_point,
                    standard_price,
                    map_price,
                    current_stock,
                    status,
                    created_at,
                    updated_at
                FROM mdm.mdm_materials
                WHERE material_id = $1
            ) t
            "#,
            material_id,
        )
        .await
    }

    async fn create_material(&self, command: CreateMaterialCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_materials (
                material_id,
                material_name,
                material_type,
                base_unit,
                default_zone,
                safety_stock,
                reorder_point,
                standard_price,
                map_price,
                current_stock,
                status
            )
            VALUES (
                $1,
                $2,
                $3::mdm.material_type,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                0,
                '正常'
            )
            RETURNING row_to_json(mdm.mdm_materials.*) AS data
            "#,
        )
        .bind(command.material_id)
        .bind(command.material_name)
        .bind(command.material_type)
        .bind(command.base_unit)
        .bind(command.default_zone)
        .bind(command.safety_stock)
        .bind(command.reorder_point)
        .bind(command.standard_price)
        .bind(command.map_price)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_materials
            SET
                material_name = COALESCE($2, material_name),
                base_unit = COALESCE($3, base_unit),
                default_zone = COALESCE($4, default_zone),
                safety_stock = COALESCE($5, safety_stock),
                reorder_point = COALESCE($6, reorder_point),
                standard_price = COALESCE($7, standard_price),
                status = COALESCE($8, status),
                updated_at = NOW()
            WHERE material_id = $1
            RETURNING row_to_json(mdm.mdm_materials.*) AS data
            "#,
        )
        .bind(material_id)
        .bind(command.material_name)
        .bind(command.base_unit)
        .bind(command.default_zone)
        .bind(command.safety_stock)
        .bind(command.reorder_point)
        .bind(command.standard_price)
        .bind(command.status)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "material not found: {material_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_material(&self, material_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_materials
            SET status = '正常', updated_at = NOW()
            WHERE material_id = $1
            "#,
        )
        .bind(material_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "material_id", material_id)
            .await
    }

    async fn deactivate_material(&self, material_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_materials
            SET status = '停用', updated_at = NOW()
            WHERE material_id = $1
            "#,
        )
        .bind(material_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "material_id", material_id)
            .await
    }
}

#[async_trait]
impl StorageBinRepository for PostgresMasterDataRepository {
    async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                bin_code,
                zone,
                bin_type,
                capacity,
                current_occupied,
                status,
                notes,
                created_at,
                updated_at
            FROM mdm.mdm_storage_bins
            ORDER BY zone, bin_code
            "#,
            query,
        )
        .await
    }

    async fn get_bin(&self, bin_code: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    bin_code,
                    zone,
                    bin_type,
                    capacity,
                    current_occupied,
                    status,
                    notes,
                    created_at,
                    updated_at
                FROM mdm.mdm_storage_bins
                WHERE bin_code = $1
            ) t
            "#,
            bin_code,
        )
        .await
    }

    async fn create_bin(&self, command: CreateStorageBinCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_storage_bins (
                bin_code,
                zone,
                bin_type,
                capacity,
                current_occupied,
                status,
                notes
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                0,
                '正常',
                $5
            )
            RETURNING row_to_json(mdm.mdm_storage_bins.*) AS data
            "#,
        )
        .bind(command.bin_code)
        .bind(command.zone)
        .bind(command.bin_type)
        .bind(command.capacity)
        .bind(command.notes)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_storage_bins
            SET
                zone = COALESCE($2, zone),
                bin_type = COALESCE($3, bin_type),
                capacity = COALESCE($4, capacity),
                status = COALESCE($5, status),
                notes = COALESCE($6, notes),
                updated_at = NOW()
            WHERE bin_code = $1
            RETURNING row_to_json(mdm.mdm_storage_bins.*) AS data
            "#,
        )
        .bind(bin_code)
        .bind(command.zone)
        .bind(command.bin_type)
        .bind(command.capacity)
        .bind(command.status)
        .bind(command.notes)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("bin not found: {bin_code}")));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_bin(&self, bin_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_storage_bins
            SET status = '正常', updated_at = NOW()
            WHERE bin_code = $1
            "#,
        )
        .bind(bin_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "bin_code", bin_code).await
    }

    async fn deactivate_bin(&self, bin_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_storage_bins
            SET status = '停用', updated_at = NOW()
            WHERE bin_code = $1
            "#,
        )
        .bind(bin_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "bin_code", bin_code).await
    }
}

#[async_trait]
impl SupplierRepository for PostgresMasterDataRepository {
    async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                supplier_id,
                supplier_name,
                contact_person,
                phone,
                email,
                address,
                quality_rating,
                is_active,
                created_at,
                updated_at
            FROM mdm.mdm_suppliers
            ORDER BY supplier_id
            "#,
            query,
        )
        .await
    }

    async fn get_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    supplier_id,
                    supplier_name,
                    contact_person,
                    phone,
                    email,
                    address,
                    quality_rating,
                    is_active,
                    created_at,
                    updated_at
                FROM mdm.mdm_suppliers
                WHERE supplier_id = $1
            ) t
            "#,
            supplier_id,
        )
        .await
    }

    async fn create_supplier(&self, command: CreateSupplierCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_suppliers (
                supplier_id,
                supplier_name,
                contact_person,
                phone,
                email,
                address,
                quality_rating,
                is_active
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                COALESCE($7, 'A'),
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_suppliers.*) AS data
            "#,
        )
        .bind(command.supplier_id)
        .bind(command.supplier_name)
        .bind(command.contact_person)
        .bind(command.phone)
        .bind(command.email)
        .bind(command.address)
        .bind(command.quality_rating)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_suppliers
            SET
                supplier_name = COALESCE($2, supplier_name),
                contact_person = COALESCE($3, contact_person),
                phone = COALESCE($4, phone),
                email = COALESCE($5, email),
                address = COALESCE($6, address),
                quality_rating = COALESCE($7, quality_rating),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE supplier_id = $1
            RETURNING row_to_json(mdm.mdm_suppliers.*) AS data
            "#,
        )
        .bind(supplier_id)
        .bind(command.supplier_name)
        .bind(command.contact_person)
        .bind(command.phone)
        .bind(command.email)
        .bind(command.address)
        .bind(command.quality_rating)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "supplier not found: {supplier_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_suppliers
            SET is_active = TRUE, updated_at = NOW()
            WHERE supplier_id = $1
            "#,
        )
        .bind(supplier_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "supplier_id", supplier_id)
            .await
    }

    async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_suppliers
            SET is_active = FALSE, updated_at = NOW()
            WHERE supplier_id = $1
            "#,
        )
        .bind(supplier_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "supplier_id", supplier_id)
            .await
    }
}

#[async_trait]
impl CustomerRepository for PostgresMasterDataRepository {
    async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                customer_id,
                customer_name,
                contact_person,
                phone,
                email,
                address,
                credit_limit,
                is_active,
                created_at,
                updated_at
            FROM mdm.mdm_customers
            ORDER BY customer_id
            "#,
            query,
        )
        .await
    }

    async fn get_customer(&self, customer_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    customer_id,
                    customer_name,
                    contact_person,
                    phone,
                    email,
                    address,
                    credit_limit,
                    is_active,
                    created_at,
                    updated_at
                FROM mdm.mdm_customers
                WHERE customer_id = $1
            ) t
            "#,
            customer_id,
        )
        .await
    }

    async fn create_customer(&self, command: CreateCustomerCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_customers (
                customer_id,
                customer_name,
                contact_person,
                phone,
                email,
                address,
                credit_limit,
                is_active
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6,
                COALESCE($7, 0),
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_customers.*) AS data
            "#,
        )
        .bind(command.customer_id)
        .bind(command.customer_name)
        .bind(command.contact_person)
        .bind(command.phone)
        .bind(command.email)
        .bind(command.address)
        .bind(command.credit_limit)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_customers
            SET
                customer_name = COALESCE($2, customer_name),
                contact_person = COALESCE($3, contact_person),
                phone = COALESCE($4, phone),
                email = COALESCE($5, email),
                address = COALESCE($6, address),
                credit_limit = COALESCE($7, credit_limit),
                is_active = COALESCE($8, is_active),
                updated_at = NOW()
            WHERE customer_id = $1
            RETURNING row_to_json(mdm.mdm_customers.*) AS data
            "#,
        )
        .bind(customer_id)
        .bind(command.customer_name)
        .bind(command.contact_person)
        .bind(command.phone)
        .bind(command.email)
        .bind(command.address)
        .bind(command.credit_limit)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "customer not found: {customer_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_customer(&self, customer_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_customers
            SET is_active = TRUE, updated_at = NOW()
            WHERE customer_id = $1
            "#,
        )
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "customer_id", customer_id)
            .await
    }

    async fn deactivate_customer(&self, customer_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_customers
            SET is_active = FALSE, updated_at = NOW()
            WHERE customer_id = $1
            "#,
        )
        .bind(customer_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "customer_id", customer_id)
            .await
    }
}

#[async_trait]
impl MaterialSupplierRepository for PostgresMasterDataRepository {
    async fn list_material_suppliers(&self, material_id: &str) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(t ORDER BY t.is_primary DESC, t.supplier_id), '[]'::json) AS data
            FROM (
                SELECT
                    ms.id,
                    ms.material_id,
                    ms.supplier_id,
                    s.supplier_name,
                    ms.is_primary,
                    ms.supplier_material_code,
                    ms.purchase_price,
                    ms.currency,
                    ms.lead_time_days,
                    ms.moq,
                    ms.quality_rating,
                    ms.is_active,
                    ms.created_at,
                    ms.updated_at
                FROM mdm.mdm_material_suppliers ms
                JOIN mdm.mdm_suppliers s ON s.supplier_id = ms.supplier_id
                WHERE ms.material_id = $1
            ) t
            "#,
        )
            .bind(material_id)
            .fetch_one(&self.pool)
            .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_material_suppliers (
                material_id,
                supplier_id,
                is_primary,
                supplier_material_code,
                purchase_price,
                currency,
                lead_time_days,
                moq,
                quality_rating,
                is_active
            )
            VALUES (
                $1,
                $2,
                COALESCE($3, FALSE),
                $4,
                $5,
                COALESCE($6, 'CNY'),
                COALESCE($7, 0),
                COALESCE($8, 1),
                COALESCE($9, 'A'),
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_material_suppliers.*) AS data
            "#,
        )
        .bind(command.material_id)
        .bind(command.supplier_id)
        .bind(command.is_primary)
        .bind(command.supplier_material_code)
        .bind(command.purchase_price)
        .bind(command.currency)
        .bind(command.lead_time_days)
        .bind(command.moq)
        .bind(command.quality_rating)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
        command: UpdateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_material_suppliers
            SET
                is_primary = COALESCE($3, is_primary),
                supplier_material_code = COALESCE($4, supplier_material_code),
                purchase_price = COALESCE($5, purchase_price),
                currency = COALESCE($6, currency),
                lead_time_days = COALESCE($7, lead_time_days),
                moq = COALESCE($8, moq),
                quality_rating = COALESCE($9, quality_rating),
                is_active = COALESCE($10, is_active),
                updated_at = NOW()
            WHERE material_id = $1
              AND supplier_id = $2
            RETURNING row_to_json(mdm.mdm_material_suppliers.*) AS data
            "#,
        )
        .bind(material_id)
        .bind(supplier_id)
        .bind(command.is_primary)
        .bind(command.supplier_material_code)
        .bind(command.purchase_price)
        .bind(command.currency)
        .bind(command.lead_time_days)
        .bind(command.moq)
        .bind(command.quality_rating)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "material supplier not found: material_id={material_id}, supplier_id={supplier_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn set_primary_supplier(&self, material_id: &str, supplier_id: &str) -> AppResult<Value> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE mdm.mdm_material_suppliers
            SET is_primary = FALSE, updated_at = NOW()
            WHERE material_id = $1
            "#,
        )
        .bind(material_id)
        .execute(&mut *tx)
        .await?;

        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_material_suppliers
            SET is_primary = TRUE, is_active = TRUE, updated_at = NOW()
            WHERE material_id = $1
              AND supplier_id = $2
            RETURNING row_to_json(mdm.mdm_material_suppliers.*) AS data
            "#,
        )
        .bind(material_id)
        .bind(supplier_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "material supplier not found: material_id={material_id}, supplier_id={supplier_id}"
            )));
        };

        tx.commit().await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            DELETE FROM mdm.mdm_material_suppliers
            WHERE material_id = $1
              AND supplier_id = $2
            "#,
        )
        .bind(material_id)
        .bind(supplier_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "material supplier not found: material_id={material_id}, supplier_id={supplier_id}"
            )));
        }

        Ok(serde_json::json!({
            "material_id": material_id,
            "supplier_id": supplier_id,
            "deleted": true
        }))
    }
}

#[async_trait]
impl ProductVariantRepository for PostgresMasterDataRepository {
    async fn list_variants(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                variant_code,
                variant_name,
                base_material_id,
                bom_id,
                standard_cost,
                is_active,
                created_at,
                updated_at
            FROM mdm.mdm_product_variants
            ORDER BY variant_code
            "#,
            query,
        )
        .await
    }

    async fn get_variant(&self, variant_code: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    variant_code,
                    variant_name,
                    base_material_id,
                    bom_id,
                    standard_cost,
                    is_active,
                    created_at,
                    updated_at
                FROM mdm.mdm_product_variants
                WHERE variant_code = $1
            ) t
            "#,
            variant_code,
        )
        .await
    }

    async fn create_variant(&self, command: CreateProductVariantCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_product_variants (
                variant_code,
                variant_name,
                base_material_id,
                bom_id,
                standard_cost,
                is_active
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_product_variants.*) AS data
            "#,
        )
        .bind(command.variant_code)
        .bind(command.variant_name)
        .bind(command.base_material_id)
        .bind(command.bom_id)
        .bind(command.standard_cost)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_product_variants
            SET
                variant_name = COALESCE($2, variant_name),
                bom_id = COALESCE($3, bom_id),
                standard_cost = COALESCE($4, standard_cost),
                is_active = COALESCE($5, is_active),
                updated_at = NOW()
            WHERE variant_code = $1
            RETURNING row_to_json(mdm.mdm_product_variants.*) AS data
            "#,
        )
        .bind(variant_code)
        .bind(command.variant_name)
        .bind(command.bom_id)
        .bind(command.standard_cost)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "product variant not found: {variant_code}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_variant(&self, variant_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_product_variants
            SET is_active = TRUE, updated_at = NOW()
            WHERE variant_code = $1
            "#,
        )
        .bind(variant_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "variant_code", variant_code)
            .await
    }

    async fn deactivate_variant(&self, variant_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_product_variants
            SET is_active = FALSE, updated_at = NOW()
            WHERE variant_code = $1
            "#,
        )
        .bind(variant_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "variant_code", variant_code)
            .await
    }
}

#[async_trait]
impl BomRepository for PostgresMasterDataRepository {
    async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                h.bom_id,
                h.bom_name,
                h.parent_material_id,
                pm.material_name AS parent_material_name,
                h.variant_code,
                h.version,
                h.base_quantity,
                h.valid_from,
                h.valid_to,
                h.status,
                h.is_active,
                h.created_by,
                h.approved_by,
                h.approved_at,
                h.notes,
                h.created_at,
                h.updated_at,
                COUNT(c.id) AS component_count
            FROM mdm.mdm_bom_headers h
            LEFT JOIN mdm.mdm_materials pm ON pm.material_id = h.parent_material_id
            LEFT JOIN mdm.mdm_bom_components c ON c.bom_id = h.bom_id
            GROUP BY
                h.bom_id,
                pm.material_name
            ORDER BY h.bom_id
            "#,
            query,
        )
        .await
    }

    async fn get_bom(&self, bom_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    h.*,
                    COALESCE(
                        json_agg(c ORDER BY c.id)
                            FILTER (WHERE c.id IS NOT NULL),
                        '[]'::json
                    ) AS components
                FROM mdm.mdm_bom_headers h
                LEFT JOIN mdm.mdm_bom_components c ON c.bom_id = h.bom_id
                WHERE h.bom_id = $1
                GROUP BY h.bom_id
            ) t
            "#,
            bom_id,
        )
        .await
    }

    async fn create_bom(&self, command: CreateBomHeaderCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_bom_headers (
                bom_id,
                bom_name,
                parent_material_id,
                variant_code,
                version,
                base_quantity,
                valid_from,
                valid_to,
                status,
                notes
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                COALESCE($6, 1),
                COALESCE($7::date, CURRENT_DATE),
                $8::date,
                COALESCE($9, '草稿'),
                $10
            )
            RETURNING row_to_json(mdm.mdm_bom_headers.*) AS data
            "#,
        )
        .bind(command.bom_id)
        .bind(command.bom_name)
        .bind(command.parent_material_id)
        .bind(command.variant_code)
        .bind(command.version)
        .bind(command.base_quantity)
        .bind(command.valid_from)
        .bind(command.valid_to)
        .bind(command.status)
        .bind(command.notes)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_bom(&self, bom_id: &str, command: UpdateBomHeaderCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_bom_headers
            SET
                bom_name = COALESCE($2, bom_name),
                variant_code = COALESCE($3, variant_code),
                version = COALESCE($4, version),
                base_quantity = COALESCE($5, base_quantity),
                valid_from = COALESCE($6::date, valid_from),
                valid_to = COALESCE($7::date, valid_to),
                status = COALESCE($8, status),
                is_active = COALESCE($9, is_active),
                notes = COALESCE($10, notes),
                updated_at = NOW()
            WHERE bom_id = $1
            RETURNING row_to_json(mdm.mdm_bom_headers.*) AS data
            "#,
        )
        .bind(bom_id)
        .bind(command.bom_name)
        .bind(command.variant_code)
        .bind(command.version)
        .bind(command.base_quantity)
        .bind(command.valid_from)
        .bind(command.valid_to)
        .bind(command.status)
        .bind(command.is_active)
        .bind(command.notes)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("bom not found: {bom_id}")));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let component_count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM mdm.mdm_bom_components
            WHERE bom_id = $1
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await?;

        if component_count == 0 {
            return Err(AppError::Validation(format!(
                "bom cannot be activated without components: {bom_id}"
            )));
        }

        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_bom_headers
            SET status = '生效', is_active = TRUE, updated_at = NOW()
            WHERE bom_id = $1
            "#,
        )
        .bind(bom_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "bom_id", bom_id).await
    }

    async fn deactivate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_bom_headers
            SET status = '失效', is_active = FALSE, updated_at = NOW()
            WHERE bom_id = $1
            "#,
        )
        .bind(bom_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "bom_id", bom_id).await
    }

    async fn list_components(&self, bom_id: &str) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(t ORDER BY t.id), '[]'::json) AS data
            FROM (
                SELECT
                    c.id,
                    c.bom_id,
                    c.parent_material_id,
                    pm.material_name AS parent_material_name,
                    c.component_material_id,
                    cm.material_name AS component_material_name,
                    c.quantity,
                    c.unit,
                    c.bom_level,
                    c.scrap_rate,
                    c.is_critical,
                    c.valid_from,
                    c.valid_to,
                    c.created_at
                FROM mdm.mdm_bom_components c
                LEFT JOIN mdm.mdm_materials pm ON pm.material_id = c.parent_material_id
                LEFT JOIN mdm.mdm_materials cm ON cm.material_id = c.component_material_id
                WHERE c.bom_id = $1
            ) t
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn add_component(&self, command: CreateBomComponentCommand) -> AppResult<Value> {
        if command.parent_material_id == command.component_material_id {
            return Err(AppError::Validation(
                "bom component cannot reference itself".to_string(),
            ));
        }

        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_bom_components (
                bom_id,
                parent_material_id,
                component_material_id,
                quantity,
                unit,
                bom_level,
                scrap_rate,
                is_critical
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                COALESCE($6, 1),
                COALESCE($7, 0),
                COALESCE($8, FALSE)
            )
            RETURNING row_to_json(mdm.mdm_bom_components.*) AS data
            "#,
        )
        .bind(command.bom_id)
        .bind(command.parent_material_id)
        .bind(command.component_material_id)
        .bind(command.quantity)
        .bind(command.unit)
        .bind(command.bom_level)
        .bind(command.scrap_rate)
        .bind(command.is_critical)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_bom_components
            SET
                quantity = COALESCE($2, quantity),
                unit = COALESCE($3, unit),
                bom_level = COALESCE($4, bom_level),
                scrap_rate = COALESCE($5, scrap_rate),
                is_critical = COALESCE($6, is_critical)
            WHERE id = $1
            RETURNING row_to_json(mdm.mdm_bom_components.*) AS data
            "#,
        )
        .bind(component_id)
        .bind(command.quantity)
        .bind(command.unit)
        .bind(command.bom_level)
        .bind(command.scrap_rate)
        .bind(command.is_critical)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "bom component not found: {component_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn remove_component(&self, component_id: i64) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            DELETE FROM mdm.mdm_bom_components
            WHERE id = $1
            "#,
        )
        .bind(component_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "bom component not found: {component_id}"
            )));
        }

        Ok(serde_json::json!({
            "component_id": component_id,
            "deleted": true
        }))
    }

    async fn get_bom_tree(&self, bom_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    h.bom_id,
                    h.bom_name,
                    h.parent_material_id,
                    h.variant_code,
                    h.version,
                    h.status,
                    h.is_active,
                    COALESCE(
                        json_agg(
                            json_build_object(
                                'id', c.id,
                                'component_material_id', c.component_material_id,
                                'component_material_name', cm.material_name,
                                'quantity', c.quantity,
                                'unit', c.unit,
                                'bom_level', c.bom_level,
                                'scrap_rate', c.scrap_rate,
                                'is_critical', c.is_critical
                            )
                            ORDER BY c.bom_level, c.id
                        ) FILTER (WHERE c.id IS NOT NULL),
                        '[]'::json
                    ) AS components
                FROM mdm.mdm_bom_headers h
                LEFT JOIN mdm.mdm_bom_components c ON c.bom_id = h.bom_id
                LEFT JOIN mdm.mdm_materials cm ON cm.material_id = c.component_material_id
                WHERE h.bom_id = $1
                GROUP BY h.bom_id
            ) t
            "#,
            bom_id,
        )
        .await
    }

    async fn validate_bom(&self, bom_id: &str) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            WITH header_exists AS (
                SELECT EXISTS (
                    SELECT 1
                    FROM mdm.mdm_bom_headers
                    WHERE bom_id = $1
                ) AS ok
            ),
            component_count AS (
                SELECT COUNT(*) AS cnt
                FROM mdm.mdm_bom_components
                WHERE bom_id = $1
            ),
            self_reference AS (
                SELECT COUNT(*) AS cnt
                FROM mdm.mdm_bom_components
                WHERE bom_id = $1
                  AND parent_material_id = component_material_id
            )
            SELECT json_build_object(
                'bom_id', $1,
                'header_exists', (SELECT ok FROM header_exists),
                'component_count', (SELECT cnt FROM component_count),
                'has_components', (SELECT cnt FROM component_count) > 0,
                'self_reference_count', (SELECT cnt FROM self_reference),
                'valid',
                    (SELECT ok FROM header_exists)
                    AND (SELECT cnt FROM component_count) > 0
                    AND (SELECT cnt FROM self_reference) = 0
            ) AS data
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(json_agg(t ORDER BY t.bom_level, t.component_material_id), '[]'::json) AS data
            FROM wms.fn_bom_explosion($1, $2, $3) t
            "#,
        )
            .bind(material_id)
            .bind(quantity)
            .bind(variant_code)
            .fetch_one(&self.pool)
            .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }
}

#[async_trait]
impl WorkCenterRepository for PostgresMasterDataRepository {
    async fn list_work_centers(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                work_center_id,
                work_center_name,
                location,
                capacity_per_day,
                efficiency,
                is_active,
                created_at
            FROM mdm.mdm_work_centers
            ORDER BY work_center_id
            "#,
            query,
        )
        .await
    }

    async fn get_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    work_center_id,
                    work_center_name,
                    location,
                    capacity_per_day,
                    efficiency,
                    is_active,
                    created_at
                FROM mdm.mdm_work_centers
                WHERE work_center_id = $1
            ) t
            "#,
            work_center_id,
        )
        .await
    }

    async fn create_work_center(&self, command: CreateWorkCenterCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_work_centers (
                work_center_id,
                work_center_name,
                location,
                capacity_per_day,
                efficiency,
                is_active
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                COALESCE($5, 100.00),
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_work_centers.*) AS data
            "#,
        )
        .bind(command.work_center_id)
        .bind(command.work_center_name)
        .bind(command.location)
        .bind(command.capacity_per_day)
        .bind(command.efficiency)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_work_centers
            SET
                work_center_name = COALESCE($2, work_center_name),
                location = COALESCE($3, location),
                capacity_per_day = COALESCE($4, capacity_per_day),
                efficiency = COALESCE($5, efficiency),
                is_active = COALESCE($6, is_active)
            WHERE work_center_id = $1
            RETURNING row_to_json(mdm.mdm_work_centers.*) AS data
            "#,
        )
        .bind(work_center_id)
        .bind(command.work_center_name)
        .bind(command.location)
        .bind(command.capacity_per_day)
        .bind(command.efficiency)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "work center not found: {work_center_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_work_centers
            SET is_active = TRUE
            WHERE work_center_id = $1
            "#,
        )
        .bind(work_center_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "work_center_id", work_center_id)
            .await
    }

    async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_work_centers
            SET is_active = FALSE
            WHERE work_center_id = $1
            "#,
        )
        .bind(work_center_id)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "work_center_id", work_center_id)
            .await
    }
}

#[async_trait]
impl QualityMasterRepository for PostgresMasterDataRepository {
    async fn list_inspection_chars(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                char_id,
                char_name,
                material_type::text AS material_type,
                inspection_type,
                method,
                standard,
                unit,
                lower_limit,
                upper_limit,
                is_critical,
                created_at
            FROM mdm.mdm_inspection_chars
            ORDER BY char_id
            "#,
            query,
        )
        .await
    }

    async fn get_inspection_char(&self, char_id: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    char_id,
                    char_name,
                    material_type::text AS material_type,
                    inspection_type,
                    method,
                    standard,
                    unit,
                    lower_limit,
                    upper_limit,
                    is_critical,
                    created_at
                FROM mdm.mdm_inspection_chars
                WHERE char_id = $1
            ) t
            "#,
            char_id,
        )
        .await
    }

    async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_inspection_chars (
                char_id,
                char_name,
                material_type,
                inspection_type,
                method,
                standard,
                unit,
                lower_limit,
                upper_limit,
                is_critical
            )
            VALUES (
                $1,
                $2,
                $3::mdm.material_type,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9,
                COALESCE($10, FALSE)
            )
            RETURNING row_to_json(mdm.mdm_inspection_chars.*) AS data
            "#,
        )
        .bind(command.char_id)
        .bind(command.char_name)
        .bind(command.material_type)
        .bind(command.inspection_type)
        .bind(command.method)
        .bind(command.standard)
        .bind(command.unit)
        .bind(command.lower_limit)
        .bind(command.upper_limit)
        .bind(command.is_critical)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_inspection_chars
            SET
                char_name = COALESCE($2, char_name),
                material_type = COALESCE($3::mdm.material_type, material_type),
                inspection_type = COALESCE($4, inspection_type),
                method = COALESCE($5, method),
                standard = COALESCE($6, standard),
                unit = COALESCE($7, unit),
                lower_limit = COALESCE($8, lower_limit),
                upper_limit = COALESCE($9, upper_limit),
                is_critical = COALESCE($10, is_critical)
            WHERE char_id = $1
            RETURNING row_to_json(mdm.mdm_inspection_chars.*) AS data
            "#,
        )
        .bind(char_id)
        .bind(command.char_name)
        .bind(command.material_type)
        .bind(command.inspection_type)
        .bind(command.method)
        .bind(command.standard)
        .bind(command.unit)
        .bind(command.lower_limit)
        .bind(command.upper_limit)
        .bind(command.is_critical)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "inspection char not found: {char_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn list_defect_codes(&self, query: MasterDataQuery) -> AppResult<Value> {
        self.fetch_list(
            r#"
            SELECT
                defect_code,
                defect_name,
                category,
                severity,
                description,
                is_active,
                created_at
            FROM mdm.mdm_defect_codes
            ORDER BY defect_code
            "#,
            query,
        )
        .await
    }

    async fn get_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        self.fetch_json_with_id(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    defect_code,
                    defect_name,
                    category,
                    severity,
                    description,
                    is_active,
                    created_at
                FROM mdm.mdm_defect_codes
                WHERE defect_code = $1
            ) t
            "#,
            defect_code,
        )
        .await
    }

    async fn create_defect_code(&self, command: CreateDefectCodeCommand) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            INSERT INTO mdm.mdm_defect_codes (
                defect_code,
                defect_name,
                category,
                severity,
                description,
                is_active
            )
            VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                TRUE
            )
            RETURNING row_to_json(mdm.mdm_defect_codes.*) AS data
            "#,
        )
        .bind(command.defect_code)
        .bind(command.defect_name)
        .bind(command.category)
        .bind(command.severity)
        .bind(command.description)
        .fetch_one(&self.pool)
        .await?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<Value> {
        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_defect_codes
            SET
                defect_name = COALESCE($2, defect_name),
                category = COALESCE($3, category),
                severity = COALESCE($4, severity),
                description = COALESCE($5, description),
                is_active = COALESCE($6, is_active)
            WHERE defect_code = $1
            RETURNING row_to_json(mdm.mdm_defect_codes.*) AS data
            "#,
        )
        .bind(defect_code)
        .bind(command.defect_name)
        .bind(command.category)
        .bind(command.severity)
        .bind(command.description)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "defect code not found: {defect_code}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn activate_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_defect_codes
            SET is_active = TRUE
            WHERE defect_code = $1
            "#,
        )
        .bind(defect_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "defect_code", defect_code)
            .await
    }

    async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<Value> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_defect_codes
            SET is_active = FALSE
            WHERE defect_code = $1
            "#,
        )
        .bind(defect_code)
        .execute(&self.pool)
        .await?;

        self.affected_to_json(result, "defect_code", defect_code)
            .await
    }
}
