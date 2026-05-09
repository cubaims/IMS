use async_trait::async_trait;
use serde_json::Value;
use sqlx::{PgPool, Row};

use cuba_shared::{AppError, AppResult};

use std::collections::HashSet;

use crate::domain::{
    Bom, BomComponent, BomHeader, BomId, BomStatus, MaterialId, VariantCode,
};
use crate::domain::{BinCode, StorageBin};

use crate::application::{
    BomRepository, CreateBomHeaderCommand, CreateCustomerCommand,
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
        let row = sqlx::query(sql)
            .fetch_one(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn fetch_json_with_id(&self, sql: &str, id: &str) -> AppResult<Value> {
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

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
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

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
        // 计划 §五.1:按物料类型筛选 + 按物料编码/名称模糊搜索 + 按 status 精确过滤
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
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
                WHERE
                    ($5::text IS NULL OR material_id ILIKE $5 OR material_name ILIKE $5)
                    AND ($6::text IS NULL OR material_type::text = $6)
                    AND ($7::text IS NULL OR status = $7)
                ORDER BY material_id
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.material_type.as_deref())
        .bind(query.status.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        self.affected_to_json(result, "material_id", material_id)
            .await
    }
}

#[async_trait]
impl StorageBinRepository for PostgresMasterDataRepository {
    async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Value> {
        // 计划 §五.2:按区域查询货位 + 按是否可用查询货位 + 模糊搜索
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
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
                WHERE
                    ($5::text IS NULL OR bin_code ILIKE $5 OR zone ILIKE $5)
                    AND ($6::text IS NULL OR zone = $6)
                    AND ($7::text IS NULL OR status = $7)
                ORDER BY zone, bin_code
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.zone.as_deref())
        .bind(query.status.as_deref())
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<Value> {
        // 计划 §五.2 / §六.2:容量不能小于当前占用。
        // 用 entity 的 change_capacity 跑这条跨字段规则:先 SELECT 当前状态,
        // 装进 StorageBin 调用,失败则不进 UPDATE。
        if let Some(new_capacity) = command.capacity {
            let current = sqlx::query(
                r#"
                SELECT zone, bin_type, capacity, current_occupied, status
                FROM mdm.mdm_storage_bins
                WHERE bin_code = $1
                "#,
            )
            .bind(bin_code)
            .fetch_optional(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?
            .ok_or_else(|| AppError::NotFound(format!("bin not found: {bin_code}")))?;

            let bin_code_vo = BinCode::new(bin_code)
                .map_err(|e| AppError::Validation(e.to_string()))?;
            let mut entity = StorageBin {
                bin_code: bin_code_vo,
                zone: current.try_get::<String, _>("zone").unwrap_or_default(),
                bin_type: current.try_get::<String, _>("bin_type").unwrap_or_default(),
                capacity: current.try_get::<i32, _>("capacity").unwrap_or(0),
                current_occupied: current.try_get::<i32, _>("current_occupied").unwrap_or(0),
                status: current
                    .try_get::<String, _>("status")
                    .unwrap_or_else(|_| "正常".to_string()),
            };
            // 触发 change_capacity 不变式 — 容量 >= 0 且 >= current_occupied
            entity
                .change_capacity(new_capacity)
                .map_err(|e| AppError::Validation(e.to_string()))?;
        }

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        self.affected_to_json(result, "bin_code", bin_code).await
    }

    async fn get_bin_capacity_utilization(&self, bin_code: &str) -> AppResult<Value> {
        // 计划 §五.2:查询货位容量利用率。
        // utilization_pct 用 numeric 在 SQL 计算,避免浮点;capacity = 0 时回 0.0。
        let row = sqlx::query(
            r#"
            SELECT row_to_json(t) AS data
            FROM (
                SELECT
                    bin_code,
                    zone,
                    capacity,
                    current_occupied,
                    CASE
                        WHEN capacity = 0 THEN 0.00::numeric
                        ELSE ROUND(current_occupied::numeric * 100 / capacity::numeric, 2)
                    END AS utilization_pct
                FROM mdm.mdm_storage_bins
                WHERE bin_code = $1
            ) t
            "#,
        )
        .bind(bin_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("bin not found: {bin_code}")));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }
}

#[async_trait]
impl SupplierRepository for PostgresMasterDataRepository {
    async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Value> {
        // 计划 §五.3:模糊搜索 + 按 is_active 过滤
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
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
                WHERE
                    ($5::text IS NULL OR supplier_id ILIKE $5 OR supplier_name ILIKE $5)
                    AND ($6::bool IS NULL OR is_active = $6)
                ORDER BY supplier_id
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        self.affected_to_json(result, "supplier_id", supplier_id)
            .await
    }
}

#[async_trait]
impl CustomerRepository for PostgresMasterDataRepository {
    async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Value> {
        // 计划 §五.5:模糊搜索 + 按 is_active 过滤
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
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
                WHERE
                    ($5::text IS NULL OR customer_id ILIKE $5 OR customer_name ILIKE $5)
                    AND ($6::bool IS NULL OR is_active = $6)
                ORDER BY customer_id
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
            .await.map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<Value> {
        // 计划 §五.4:停用物料不能新增供应商关系、停用供应商不能设为主供应商。
        // 这些是跨表规则,放到事务里检查避免 TOCTOU。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) 物料必须存在且未停用
        let material_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status::text
            FROM mdm.mdm_materials
            WHERE material_id = $1
            FOR SHARE
            "#,
        )
        .bind(&command.material_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match material_status {
            None => {
                return Err(AppError::Validation(format!(
                    "物料不存在: {}",
                    command.material_id
                )));
            }
            Some(s) if s == "停用" => {
                return Err(AppError::Validation(format!(
                    "物料已停用,不能新增供应商关系: {}",
                    command.material_id
                )));
            }
            _ => {}
        }

        // 2) 供应商必须存在且 is_active=TRUE
        let supplier_active: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT is_active
            FROM mdm.mdm_suppliers
            WHERE supplier_id = $1
            FOR SHARE
            "#,
        )
        .bind(&command.supplier_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match supplier_active {
            None => {
                return Err(AppError::Validation(format!(
                    "供应商不存在: {}",
                    command.supplier_id
                )));
            }
            Some(false) => {
                return Err(AppError::Validation(format!(
                    "供应商已停用,不能设为主供应商: {}",
                    command.supplier_id
                )));
            }
            _ => {}
        }

        // 3) 如果命令里 is_primary=TRUE,先把这个物料原有的所有 primary 清掉,
        //    保证"一个物料只能有一个主供应商"不被破坏。
        let want_primary = command.is_primary.unwrap_or(false);
        if want_primary {
            sqlx::query(
                r#"
                UPDATE mdm.mdm_material_suppliers
                SET is_primary = FALSE, updated_at = NOW()
                WHERE material_id = $1 AND is_primary = TRUE
                "#,
            )
            .bind(&command.material_id)
            .execute(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
        }

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
        .fetch_one(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!(
                "material supplier not found: material_id={material_id}, supplier_id={supplier_id}"
            )));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn set_primary_supplier(&self, material_id: &str, supplier_id: &str) -> AppResult<Value> {
        // 计划 §五.4:停用供应商不能设为主供应商。
        // 全程在事务里:先校验供应商 active + 物料-供应商关系 active,
        // 再原子地清旧主、设新主。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) 供应商必须 active
        let supplier_active: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT is_active FROM mdm.mdm_suppliers
            WHERE supplier_id = $1
            FOR SHARE
            "#,
        )
        .bind(supplier_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match supplier_active {
            None => {
                return Err(AppError::Validation(format!("供应商不存在: {supplier_id}")));
            }
            Some(false) => {
                return Err(AppError::Validation(format!(
                    "供应商已停用,不能设为主供应商: {supplier_id}"
                )));
            }
            _ => {}
        }

        // 2) 目标物料-供应商关系必须存在,且自身 is_active=TRUE
        //    (不再像之前那样自动 reactivate,显式风险更小)
        let rel_active: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT is_active FROM mdm.mdm_material_suppliers
            WHERE material_id = $1 AND supplier_id = $2
            FOR UPDATE
            "#,
        )
        .bind(material_id)
        .bind(supplier_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match rel_active {
            None => {
                return Err(AppError::NotFound(format!(
                    "material supplier not found: material_id={material_id}, supplier_id={supplier_id}"
                )));
            }
            Some(false) => {
                return Err(AppError::Validation(format!(
                    "物料-供应商关系已禁用,请先 PATCH 把 is_active 设为 true 再 set primary"
                )));
            }
            _ => {}
        }

        // 3) 清掉这个物料原有的 primary,再把目标设为 primary。
        //    "一个物料只能有一个主供应商"由这两步原子完成。
        sqlx::query(
            r#"
            UPDATE mdm.mdm_material_suppliers
            SET is_primary = FALSE, updated_at = NOW()
            WHERE material_id = $1 AND is_primary = TRUE
            "#,
        )
        .bind(material_id)
        .execute(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let row = sqlx::query(
            r#"
            UPDATE mdm.mdm_material_suppliers
            SET is_primary = TRUE, updated_at = NOW()
            WHERE material_id = $1 AND supplier_id = $2
            RETURNING row_to_json(mdm.mdm_material_suppliers.*) AS data
            "#,
        )
        .bind(material_id)
        .bind(supplier_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        // 计划 §五.6:支持模糊搜索 + is_active 过滤
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
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
                WHERE
                    ($5::text IS NULL OR variant_code ILIKE $5 OR variant_name ILIKE $5)
                    AND ($6::bool IS NULL OR is_active = $6)
                ORDER BY variant_code
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        // 计划 §五.6 领域规则:
        //   - 变体必须绑定有效成品物料(base_material_id 必须存在且 status='正常')
        //   - 绑定 BOM 必须存在且有效(bom_id 可选,如果给了必须 is_active=TRUE)
        // 用 FOR SHARE 锁住引用行,防 TOCTOU。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) base_material 必须存在且未停用
        let mat_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status::text
            FROM mdm.mdm_materials
            WHERE material_id = $1
            FOR SHARE
            "#,
        )
        .bind(&command.base_material_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match mat_status {
            None => {
                return Err(AppError::Validation(format!(
                    "base_material 不存在: {}",
                    command.base_material_id
                )));
            }
            Some(s) if s == "停用" => {
                return Err(AppError::Validation(format!(
                    "base_material 已停用,不能用于产品变体: {}",
                    command.base_material_id
                )));
            }
            _ => {}
        }

        // 2) 如果指定了 bom_id,该 BOM 必须存在且 is_active=TRUE
        if let Some(bom_id) = command.bom_id.as_deref() {
            let bom_active: Option<bool> = sqlx::query_scalar(
                r#"
                SELECT is_active
                FROM mdm.mdm_bom_headers
                WHERE bom_id = $1
                FOR SHARE
                "#,
            )
            .bind(bom_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
            match bom_active {
                None => {
                    return Err(AppError::Validation(format!("bom_id 不存在: {bom_id}")));
                }
                Some(false) => {
                    return Err(AppError::Validation(format!(
                        "bom_id 未生效,不能绑定到产品变体: {bom_id}"
                    )));
                }
                _ => {}
            }
        }

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
        .fetch_one(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        self.affected_to_json(result, "variant_code", variant_code)
            .await
    }
}

#[async_trait]
impl BomRepository for PostgresMasterDataRepository {
    async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Value> {
        // 计划 §五.7:模糊搜索 (bom_id / bom_name) + status + is_active 过滤
        let page = query.page.unwrap_or(1).max(1) as i32;
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200) as i32;
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let row = sqlx::query(
            r#"
            SELECT json_build_object(
                'items', COALESCE(json_agg(t), '[]'::json),
                'page', $1::int,
                'page_size', $2::int
            ) AS data
            FROM (
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
                WHERE
                    ($5::text IS NULL OR h.bom_id ILIKE $5 OR h.bom_name ILIKE $5)
                    AND ($6::text IS NULL OR h.status = $6)
                    AND ($7::bool IS NULL OR h.is_active = $7)
                GROUP BY
                    h.bom_id,
                    pm.material_name
                ORDER BY h.bom_id
                LIMIT $3 OFFSET $4
            ) t
            "#,
        )
        .bind(page)
        .bind(page_size)
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.status.as_deref())
        .bind(query.is_active)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::NotFound(format!("bom not found: {bom_id}")));
        };

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        // 计划 §五.7 BOM 校验:
        //   - header_exists(BOM 是否存在)
        //   - has_components(组件数 > 0)
        //   - self_reference_count(自引用条数,应为 0)
        //   - missing_component_materials(组件物料不存在的条数,应为 0)
        //   - cycle_detected(整个 BOM 图(含本 BOM 假设激活)是否有循环)
        //   - cycle_node(若 cycle_detected,给出一个环上的节点便于前端定位)
        let row = sqlx::query(
            r#"
            WITH header_exists AS (
                SELECT EXISTS (
                    SELECT 1 FROM mdm.mdm_bom_headers WHERE bom_id = $1
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
            ),
            missing_components AS (
                SELECT COUNT(*) AS cnt
                FROM mdm.mdm_bom_components c
                WHERE c.bom_id = $1
                  AND NOT EXISTS (
                    SELECT 1 FROM mdm.mdm_materials m
                    WHERE m.material_id = c.component_material_id
                  )
            ),
            cycle_walk AS (
                WITH RECURSIVE walk(start_node, current_node, depth) AS (
                    SELECT c.parent_material_id, c.parent_material_id, 0
                    FROM mdm.mdm_bom_components c
                    JOIN mdm.mdm_bom_headers h ON h.bom_id = c.bom_id
                    WHERE h.is_active = TRUE OR h.bom_id = $1
                    UNION ALL
                    SELECT w.start_node, c.component_material_id, w.depth + 1
                    FROM mdm.mdm_bom_components c
                    JOIN mdm.mdm_bom_headers h ON h.bom_id = c.bom_id
                    JOIN walk w ON c.parent_material_id = w.current_node
                    WHERE (h.is_active = TRUE OR h.bom_id = $1)
                      AND w.depth < 50
                      AND w.current_node <> w.start_node
                )
                SELECT start_node
                FROM walk
                WHERE current_node = start_node AND depth > 0
                LIMIT 1
            )
            SELECT json_build_object(
                'bom_id', $1,
                'header_exists', (SELECT ok FROM header_exists),
                'component_count', (SELECT cnt FROM component_count),
                'has_components', (SELECT cnt FROM component_count) > 0,
                'self_reference_count', (SELECT cnt FROM self_reference),
                'missing_component_materials', (SELECT cnt FROM missing_components),
                'cycle_detected', EXISTS (SELECT 1 FROM cycle_walk),
                'cycle_node', (SELECT start_node FROM cycle_walk),
                'valid',
                    (SELECT ok FROM header_exists)
                    AND (SELECT cnt FROM component_count) > 0
                    AND (SELECT cnt FROM self_reference) = 0
                    AND (SELECT cnt FROM missing_components) = 0
                    AND NOT EXISTS (SELECT 1 FROM cycle_walk)
            ) AS data
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
            .await.map_err(cuba_shared::map_master_data_db_error)?;

        row.try_get::<Value, _>("data")
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    async fn load_bom(&self, bom_id: &BomId) -> AppResult<Bom> {
        use sqlx::Row;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) header
        let header_row = sqlx::query(
            r#"
            SELECT bom_id, bom_name, parent_material_id,
                   variant_code, version, status, is_active
            FROM mdm.mdm_bom_headers
            WHERE bom_id = $1
            "#,
        )
            .bind(bom_id.value())
            .fetch_optional(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?
            .ok_or_else(|| {
                AppError::business("BOM_NOT_FOUND", format!("BOM 不存在: {}", bom_id.value()))
            })?;

        let header = BomHeader {
            bom_id: BomId::new(header_row.get::<String, _>("bom_id"))?,
            bom_name: header_row.get::<String, _>("bom_name"),
            parent_material_id: MaterialId::new(
                header_row.get::<String, _>("parent_material_id"),
            )?,
            variant_code: header_row
                .get::<Option<String>, _>("variant_code")
                .map(VariantCode::new)
                .transpose()?,
            version: header_row.get::<String, _>("version"),
            status: BomStatus::from_db_value(&header_row.get::<String, _>("status"))
                .ok_or_else(|| AppError::Validation("未知的 BOM 状态值".to_string()))?,
            is_active: header_row.get::<bool, _>("is_active"),
        };

        // 2) components
        let component_rows = sqlx::query(
            r#"
            SELECT bom_id, parent_material_id, component_material_id,
                   quantity, unit, bom_level, scrap_rate, is_critical
            FROM mdm.mdm_bom_components
            WHERE bom_id = $1
            ORDER BY id
            "#,
        )
            .bind(bom_id.value())
            .fetch_all(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        let mut components = Vec::with_capacity(component_rows.len());
        for r in component_rows {
            components.push(BomComponent {
                bom_id: BomId::new(r.get::<String, _>("bom_id"))?,
                parent_material_id: MaterialId::new(
                    r.get::<String, _>("parent_material_id"),
                )?,
                component_material_id: MaterialId::new(
                    r.get::<String, _>("component_material_id"),
                )?,
                quantity: r.get::<rust_decimal::Decimal, _>("quantity"),
                unit: r.get::<String, _>("unit"),
                bom_level: r.get::<i32, _>("bom_level"),
                scrap_rate: r.get::<rust_decimal::Decimal, _>("scrap_rate"),
                is_critical: r.get::<bool, _>("is_critical"),
            });
        }

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        Ok(Bom::from_storage(header, components))
    }

    async fn save_bom(&self, bom: &Bom) -> AppResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) UPDATE header(create_bom 已经把 row 建好,这里只更新可变字段)
        sqlx::query(
            r#"
            UPDATE mdm.mdm_bom_headers
            SET bom_name     = $2,
                variant_code = $3,
                version      = $4,
                status       = $5,
                is_active    = $6,
                updated_at   = NOW()
            WHERE bom_id = $1
            "#,
        )
            .bind(bom.id().value())
            .bind(&bom.header().bom_name)
            .bind(bom.header().variant_code.as_ref().map(|v| v.value()))
            .bind(&bom.header().version)
            .bind(bom.header().status.as_db_value())
            .bind(bom.header().is_active)
            .execute(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 2) Diff components by (parent, component) edge
        let existing: Vec<(String, String)> = sqlx::query_as(
            r#"
            SELECT parent_material_id, component_material_id
            FROM mdm.mdm_bom_components
            WHERE bom_id = $1
            "#,
        )
            .bind(bom.id().value())
            .fetch_all(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        let existing_set: HashSet<(String, String)> = existing.into_iter().collect();
        let desired_set: HashSet<(String, String)> = bom
            .components()
            .iter()
            .map(|c| {
                (
                    c.parent_material_id.value().to_string(),
                    c.component_material_id.value().to_string(),
                )
            })
            .collect();

        // 2a) DELETE 多余边
        for (parent, child) in existing_set.difference(&desired_set) {
            sqlx::query(
                r#"
                DELETE FROM mdm.mdm_bom_components
                WHERE bom_id = $1
                  AND parent_material_id = $2
                  AND component_material_id = $3
                "#,
            )
                .bind(bom.id().value())
                .bind(parent)
                .bind(child)
                .execute(&mut *tx)
                .await
                .map_err(cuba_shared::map_master_data_db_error)?;
        }

        // 2b) INSERT 新边 / UPDATE 已有边的可变属性
        for c in bom.components() {
            let parent = c.parent_material_id.value().to_string();
            let child = c.component_material_id.value().to_string();
            let edge = (parent.clone(), child.clone());

            if existing_set.contains(&edge) {
                sqlx::query(
                    r#"
                    UPDATE mdm.mdm_bom_components
                    SET quantity    = $4,
                        unit        = $5,
                        bom_level   = $6,
                        scrap_rate  = $7,
                        is_critical = $8,
                        updated_at  = NOW()
                    WHERE bom_id = $1
                      AND parent_material_id = $2
                      AND component_material_id = $3
                    "#,
                )
                    .bind(bom.id().value())
                    .bind(&parent)
                    .bind(&child)
                    .bind(c.quantity)
                    .bind(&c.unit)
                    .bind(c.bom_level)
                    .bind(c.scrap_rate)
                    .bind(c.is_critical)
                    .execute(&mut *tx)
                    .await
                    .map_err(cuba_shared::map_master_data_db_error)?;
            } else {
                sqlx::query(
                    r#"
                    INSERT INTO mdm.mdm_bom_components
                        (bom_id, parent_material_id, component_material_id,
                         quantity, unit, bom_level, scrap_rate, is_critical,
                         created_at, updated_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW(), NOW())
                    "#,
                )
                    .bind(bom.id().value())
                    .bind(&parent)
                    .bind(&child)
                    .bind(c.quantity)
                    .bind(&c.unit)
                    .bind(c.bom_level)
                    .bind(c.scrap_rate)
                    .bind(c.is_critical)
                    .execute(&mut *tx)
                    .await
                    .map_err(cuba_shared::map_master_data_db_error)?;
            }
        }

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
        Ok(())
    }

    async fn assert_no_cycle_after_change(&self, bom: &Bom) -> AppResult<()> {
        // 0 个组件不可能引入新环,直接放行
        if bom.components().is_empty() {
            return Ok(());
        }

        // 把聚合的边拆成两个并行数组,通过 UNNEST 喂给 SQL
        let parents: Vec<String> = bom
            .components()
            .iter()
            .map(|c| c.parent_material_id.value().to_string())
            .collect();
        let children: Vec<String> = bom
            .components()
            .iter()
            .map(|c| c.component_material_id.value().to_string())
            .collect();

        let cycle: Option<String> = sqlx::query_scalar(
            r#"
            WITH this_bom_edges AS (
                SELECT * FROM UNNEST($2::TEXT[], $3::TEXT[]) AS t(parent_id, child_id)
            ),
            other_bom_edges AS (
                SELECT c.parent_material_id AS parent_id,
                       c.component_material_id AS child_id
                FROM mdm.mdm_bom_components c
                JOIN mdm.mdm_bom_headers h ON h.bom_id = c.bom_id
                WHERE c.bom_id <> $1
                  AND h.is_active = TRUE
            ),
            all_edges AS (
                SELECT parent_id, child_id FROM this_bom_edges
                UNION
                SELECT parent_id, child_id FROM other_bom_edges
            ),
            walk(start_node, current_node, depth) AS (
                SELECT parent_id, parent_id, 0 FROM all_edges
                UNION ALL
                SELECT w.start_node, e.child_id, w.depth + 1
                FROM all_edges e
                JOIN walk w ON e.parent_id = w.current_node
                WHERE w.depth < 50
                  AND w.current_node <> w.start_node
            )
            SELECT start_node FROM walk
            WHERE current_node = start_node AND depth > 0
            LIMIT 1
            "#,
        )
            .bind(bom.id().value())
            .bind(&parents)
            .bind(&children)
            .fetch_optional(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        if let Some(node) = cycle {
            return Err(AppError::business(
                "BOM_CYCLE_DETECTED",
                format!("BOM 存在循环引用,环路涉及物料 {node}"),
            ));
        }
        Ok(())
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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

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
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        self.affected_to_json(result, "defect_code", defect_code)
            .await
    }
}
