use async_trait::async_trait;
use sqlx::{Decode, PgPool, Postgres, Row, Type, postgres::PgRow};

use cuba_shared::{AppError, AppResult, Page};

use std::collections::HashSet;

use crate::domain::{
    BinCode, DefectSeverity, InspectionCharId, InspectionCharacteristic, MaterialType, StorageBin,
};
use crate::domain::{Bom, BomComponent, BomHeader, BomId, BomStatus, MaterialId, VariantCode};

use crate::application::{
    BinCapacityUtilizationReadModel, BomComponentReadModel, BomDetailReadModel,
    BomExplosionItemReadModel, BomExplosionPreviewReadModel, BomHeaderReadModel, BomRepository,
    BomSummaryReadModel, BomTreeComponentReadModel, BomTreeReadModel, BomValidationReadModel,
    CreateBomHeaderCommand, CreateCustomerCommand, CreateDefectCodeCommand,
    CreateInspectionCharCommand, CreateMaterialCommand, CreateMaterialSupplierCommand,
    CreateProductVariantCommand, CreateStorageBinCommand, CreateSupplierCommand,
    CreateWorkCenterCommand, CustomerReadModel, CustomerRepository, DefectCodeReadModel, DeleteAck,
    InspectionCharacteristicReadModel, MasterDataQuery, MaterialReadModel, MaterialRepository,
    MaterialSupplierReadModel, MaterialSupplierRepository, MutationAck, ProductVariantReadModel,
    ProductVariantRepository, QualityMasterRepository, StorageBinReadModel, StorageBinRepository,
    SupplierReadModel, SupplierRepository, UpdateBomComponentCommand, UpdateBomHeaderCommand,
    UpdateCustomerCommand, UpdateDefectCodeCommand, UpdateInspectionCharCommand,
    UpdateMaterialCommand, UpdateMaterialSupplierCommand, UpdateProductVariantCommand,
    UpdateStorageBinCommand, UpdateSupplierCommand, UpdateWorkCenterCommand, WorkCenterReadModel,
    WorkCenterRepository,
};

#[derive(Clone)]
pub struct PostgresMasterDataRepository {
    pool: PgPool,
}

impl PostgresMasterDataRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn column<T>(row: &PgRow, name: &str) -> AppResult<T>
    where
        for<'r> T: Decode<'r, Postgres> + Type<Postgres>,
    {
        row.try_get(name)
            .map_err(|error| AppError::Internal(error.to_string()))
    }

    fn pagination(query: &MasterDataQuery) -> (u64, u64, i64, i64) {
        let page = u64::from(query.page.unwrap_or(1).max(1));
        let page_size = u64::from(query.page_size.unwrap_or(20).clamp(1, 200));
        let limit = page_size as i64;
        let offset = ((page - 1) as i64) * limit;

        (page, page_size, limit, offset)
    }

    fn page_from_rows<T>(
        rows: Vec<PgRow>,
        page: u64,
        page_size: u64,
        parse: fn(&PgRow) -> AppResult<T>,
    ) -> AppResult<Page<T>> {
        let total = rows
            .first()
            .map(|row| Self::column::<i64>(row, "total"))
            .transpose()?
            .unwrap_or(0);
        let items = rows.iter().map(parse).collect::<AppResult<Vec<_>>>()?;

        Ok(Page::new(items, total.max(0) as u64, page, page_size))
    }

    async fn fetch_one_by_id<T>(
        &self,
        sql: &str,
        id: &str,
        not_found_code: &'static str,
        not_found_message: impl Into<String>,
        parse: fn(&PgRow) -> AppResult<T>,
    ) -> AppResult<T> {
        let row = sqlx::query(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(not_found_code, not_found_message));
        };

        parse(&row)
    }

    fn normalize_material_type(value: &str) -> AppResult<&'static str> {
        MaterialType::from_db_value(value)
            .map(|material_type| material_type.as_db_value())
            .ok_or_else(|| {
                AppError::Validation(format!("未知的物料类型 '{value}',应为 原材料/半成品/成品"))
            })
    }

    fn normalize_optional_material_type(value: Option<&str>) -> AppResult<Option<&'static str>> {
        value.map(Self::normalize_material_type).transpose()
    }

    fn normalize_bom_status(value: &str) -> AppResult<&'static str> {
        BomStatus::from_db_value(value)
            .map(|status| status.as_db_value())
            .ok_or_else(|| {
                AppError::Validation(format!("未知的 BOM 状态 '{value}',应为 草稿/生效/失效"))
            })
    }

    fn normalize_optional_bom_status(value: Option<&str>) -> AppResult<Option<&'static str>> {
        value.map(Self::normalize_bom_status).transpose()
    }

    fn normalize_defect_severity(value: &str) -> AppResult<&'static str> {
        DefectSeverity::from_db_value(value)
            .map(|severity| severity.as_db_value())
            .ok_or_else(|| {
                AppError::Validation(format!("未知的不良严重等级 '{value}',应为 一般/严重/紧急"))
            })
    }

    fn normalize_optional_defect_severity(value: Option<&str>) -> AppResult<Option<&'static str>> {
        value.map(Self::normalize_defect_severity).transpose()
    }

    fn affected_to_ack(
        result: sqlx::postgres::PgQueryResult,
        id_value: &str,
        not_found_code: &'static str,
        not_found_message: impl Into<String>,
    ) -> AppResult<MutationAck> {
        if result.rows_affected() == 0 {
            return Err(AppError::business(not_found_code, not_found_message));
        }

        Ok(MutationAck {
            resource_id: id_value.to_string(),
            affected: result.rows_affected(),
        })
    }

    async fn ensure_bom_exists(&self, bom_id: &str) -> AppResult<()> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS (
                SELECT 1 FROM mdm.mdm_bom_headers WHERE bom_id = $1
            )
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        if !exists {
            return Err(AppError::business(
                "BOM_NOT_FOUND",
                format!("BOM 不存在: {bom_id}"),
            ));
        }

        Ok(())
    }

    fn parse_material(row: &PgRow) -> AppResult<MaterialReadModel> {
        Ok(MaterialReadModel {
            material_id: Self::column(row, "material_id")?,
            material_name: Self::column(row, "material_name")?,
            material_type: Self::column(row, "material_type")?,
            base_unit: Self::column(row, "base_unit")?,
            default_zone: Self::column(row, "default_zone")?,
            safety_stock: Self::column(row, "safety_stock")?,
            reorder_point: Self::column(row, "reorder_point")?,
            standard_price: Self::column(row, "standard_price")?,
            map_price: Self::column(row, "map_price")?,
            current_stock: Self::column(row, "current_stock")?,
            status: Self::column(row, "status")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_storage_bin(row: &PgRow) -> AppResult<StorageBinReadModel> {
        Ok(StorageBinReadModel {
            bin_code: Self::column(row, "bin_code")?,
            zone: Self::column(row, "zone")?,
            bin_type: Self::column(row, "bin_type")?,
            capacity: Self::column(row, "capacity")?,
            current_occupied: Self::column(row, "current_occupied")?,
            status: Self::column(row, "status")?,
            notes: Self::column(row, "notes")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_bin_utilization(row: &PgRow) -> AppResult<BinCapacityUtilizationReadModel> {
        Ok(BinCapacityUtilizationReadModel {
            bin_code: Self::column(row, "bin_code")?,
            zone: Self::column(row, "zone")?,
            capacity: Self::column(row, "capacity")?,
            current_occupied: Self::column(row, "current_occupied")?,
            utilization_pct: Self::column(row, "utilization_pct")?,
        })
    }

    fn parse_supplier(row: &PgRow) -> AppResult<SupplierReadModel> {
        Ok(SupplierReadModel {
            supplier_id: Self::column(row, "supplier_id")?,
            supplier_name: Self::column(row, "supplier_name")?,
            contact_person: Self::column(row, "contact_person")?,
            phone: Self::column(row, "phone")?,
            email: Self::column(row, "email")?,
            address: Self::column(row, "address")?,
            quality_rating: Self::column(row, "quality_rating")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_customer(row: &PgRow) -> AppResult<CustomerReadModel> {
        Ok(CustomerReadModel {
            customer_id: Self::column(row, "customer_id")?,
            customer_name: Self::column(row, "customer_name")?,
            contact_person: Self::column(row, "contact_person")?,
            phone: Self::column(row, "phone")?,
            email: Self::column(row, "email")?,
            address: Self::column(row, "address")?,
            credit_limit: Self::column(row, "credit_limit")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_material_supplier(row: &PgRow) -> AppResult<MaterialSupplierReadModel> {
        Ok(MaterialSupplierReadModel {
            id: Self::column(row, "id")?,
            material_id: Self::column(row, "material_id")?,
            supplier_id: Self::column(row, "supplier_id")?,
            supplier_name: Self::column(row, "supplier_name")?,
            is_primary: Self::column(row, "is_primary")?,
            supplier_material_code: Self::column(row, "supplier_material_code")?,
            purchase_price: Self::column(row, "purchase_price")?,
            currency: Self::column(row, "currency")?,
            lead_time_days: Self::column(row, "lead_time_days")?,
            moq: Self::column(row, "moq")?,
            quality_rating: Self::column(row, "quality_rating")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_variant(row: &PgRow) -> AppResult<ProductVariantReadModel> {
        Ok(ProductVariantReadModel {
            variant_code: Self::column(row, "variant_code")?,
            variant_name: Self::column(row, "variant_name")?,
            base_material_id: Self::column(row, "base_material_id")?,
            bom_id: Self::column(row, "bom_id")?,
            standard_cost: Self::column(row, "standard_cost")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_bom_summary(row: &PgRow) -> AppResult<BomSummaryReadModel> {
        Ok(BomSummaryReadModel {
            bom_id: Self::column(row, "bom_id")?,
            bom_name: Self::column(row, "bom_name")?,
            parent_material_id: Self::column(row, "parent_material_id")?,
            parent_material_name: Self::column(row, "parent_material_name")?,
            variant_code: Self::column(row, "variant_code")?,
            version: Self::column(row, "version")?,
            base_quantity: Self::column(row, "base_quantity")?,
            valid_from: Self::column(row, "valid_from")?,
            valid_to: Self::column(row, "valid_to")?,
            status: Self::column(row, "status")?,
            is_active: Self::column(row, "is_active")?,
            created_by: Self::column(row, "created_by")?,
            approved_by: Self::column(row, "approved_by")?,
            approved_at: Self::column(row, "approved_at")?,
            notes: Self::column(row, "notes")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
            component_count: Self::column(row, "component_count")?,
        })
    }

    fn parse_bom_header(row: &PgRow) -> AppResult<BomHeaderReadModel> {
        Ok(BomHeaderReadModel {
            bom_id: Self::column(row, "bom_id")?,
            bom_name: Self::column(row, "bom_name")?,
            parent_material_id: Self::column(row, "parent_material_id")?,
            variant_code: Self::column(row, "variant_code")?,
            version: Self::column(row, "version")?,
            base_quantity: Self::column(row, "base_quantity")?,
            valid_from: Self::column(row, "valid_from")?,
            valid_to: Self::column(row, "valid_to")?,
            status: Self::column(row, "status")?,
            is_active: Self::column(row, "is_active")?,
            created_by: Self::column(row, "created_by")?,
            approved_by: Self::column(row, "approved_by")?,
            approved_at: Self::column(row, "approved_at")?,
            notes: Self::column(row, "notes")?,
            created_at: Self::column(row, "created_at")?,
            updated_at: Self::column(row, "updated_at")?,
        })
    }

    fn parse_bom_component(row: &PgRow) -> AppResult<BomComponentReadModel> {
        Ok(BomComponentReadModel {
            id: Self::column(row, "id")?,
            bom_id: Self::column(row, "bom_id")?,
            parent_material_id: Self::column(row, "parent_material_id")?,
            parent_material_name: Self::column(row, "parent_material_name")?,
            component_material_id: Self::column(row, "component_material_id")?,
            component_material_name: Self::column(row, "component_material_name")?,
            quantity: Self::column(row, "quantity")?,
            unit: Self::column(row, "unit")?,
            bom_level: Self::column(row, "bom_level")?,
            scrap_rate: Self::column(row, "scrap_rate")?,
            is_critical: Self::column(row, "is_critical")?,
            valid_from: Self::column(row, "valid_from")?,
            valid_to: Self::column(row, "valid_to")?,
            created_at: Self::column(row, "created_at")?,
        })
    }

    fn parse_bom_tree_component(row: &PgRow) -> AppResult<BomTreeComponentReadModel> {
        Ok(BomTreeComponentReadModel {
            id: Self::column(row, "id")?,
            component_material_id: Self::column(row, "component_material_id")?,
            component_material_name: Self::column(row, "component_material_name")?,
            quantity: Self::column(row, "quantity")?,
            unit: Self::column(row, "unit")?,
            bom_level: Self::column(row, "bom_level")?,
            scrap_rate: Self::column(row, "scrap_rate")?,
            is_critical: Self::column(row, "is_critical")?,
        })
    }

    fn parse_validation(row: &PgRow) -> AppResult<BomValidationReadModel> {
        Ok(BomValidationReadModel {
            bom_id: Self::column(row, "bom_id")?,
            header_exists: Self::column(row, "header_exists")?,
            component_count: Self::column(row, "component_count")?,
            has_components: Self::column(row, "has_components")?,
            self_reference_count: Self::column(row, "self_reference_count")?,
            missing_component_materials: Self::column(row, "missing_component_materials")?,
            cycle_detected: Self::column(row, "cycle_detected")?,
            cycle_node: Self::column(row, "cycle_node")?,
            valid: Self::column(row, "valid")?,
        })
    }

    fn parse_explosion_item(row: &PgRow) -> AppResult<BomExplosionItemReadModel> {
        Ok(BomExplosionItemReadModel {
            bom_level: Self::column(row, "bom_level")?,
            parent_material_id: Self::column(row, "parent_material_id")?,
            component_material_id: Self::column(row, "component_material_id")?,
            component_name: Self::column(row, "component_name")?,
            unit_qty: Self::column(row, "unit_qty")?,
            required_qty: Self::column(row, "required_qty")?,
            available_qty: Self::column(row, "available_qty")?,
            shortage_qty: Self::column(row, "shortage_qty")?,
            is_critical: Self::column(row, "is_critical")?,
        })
    }

    fn parse_work_center(row: &PgRow) -> AppResult<WorkCenterReadModel> {
        Ok(WorkCenterReadModel {
            work_center_id: Self::column(row, "work_center_id")?,
            work_center_name: Self::column(row, "work_center_name")?,
            location: Self::column(row, "location")?,
            capacity_per_day: Self::column(row, "capacity_per_day")?,
            efficiency: Self::column(row, "efficiency")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
        })
    }

    fn parse_inspection_char(row: &PgRow) -> AppResult<InspectionCharacteristicReadModel> {
        Ok(InspectionCharacteristicReadModel {
            char_id: Self::column(row, "char_id")?,
            char_name: Self::column(row, "char_name")?,
            material_type: Self::column(row, "material_type")?,
            inspection_type: Self::column(row, "inspection_type")?,
            method: Self::column(row, "method")?,
            standard: Self::column(row, "standard")?,
            unit: Self::column(row, "unit")?,
            lower_limit: Self::column(row, "lower_limit")?,
            upper_limit: Self::column(row, "upper_limit")?,
            is_critical: Self::column(row, "is_critical")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
        })
    }

    fn parse_defect_code(row: &PgRow) -> AppResult<DefectCodeReadModel> {
        Ok(DefectCodeReadModel {
            defect_code: Self::column(row, "defect_code")?,
            defect_name: Self::column(row, "defect_name")?,
            category: Self::column(row, "category")?,
            severity: Self::column(row, "severity")?,
            description: Self::column(row, "description")?,
            is_active: Self::column(row, "is_active")?,
            created_at: Self::column(row, "created_at")?,
        })
    }

    async fn fetch_bom_components(&self, bom_id: &str) -> AppResult<Vec<BomComponentReadModel>> {
        let rows = sqlx::query(
            r#"
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
            ORDER BY c.id
            "#,
        )
        .bind(bom_id)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        rows.iter()
            .map(Self::parse_bom_component)
            .collect::<AppResult<Vec<_>>>()
    }
}

#[async_trait]
impl MaterialRepository for PostgresMasterDataRepository {
    async fn list_materials(&self, query: MasterDataQuery) -> AppResult<Page<MaterialReadModel>> {
        // 计划 §五.1:按物料类型筛选 + 按物料编码/名称模糊搜索 + 按 status 精确过滤
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));
        let material_type = Self::normalize_optional_material_type(query.material_type.as_deref())?;

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR material_id ILIKE $3 OR material_name ILIKE $3)
                AND ($4::text IS NULL OR material_type::text = $4)
                AND ($5::text IS NULL OR status = $5)
            ORDER BY material_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(material_type)
        .bind(query.status.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_material)
    }

    async fn get_material(&self, material_id: &str) -> AppResult<MaterialReadModel> {
        self.fetch_one_by_id(
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
            WHERE material_id = $1
            "#,
            material_id,
            "MATERIAL_NOT_FOUND",
            format!("物料不存在: {material_id}"),
            Self::parse_material,
        )
        .await
    }

    async fn create_material(
        &self,
        command: CreateMaterialCommand,
    ) -> AppResult<MaterialReadModel> {
        let material_type = Self::normalize_material_type(&command.material_type)?;

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
            RETURNING
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
            "#,
        )
        .bind(command.material_id)
        .bind(command.material_name)
        .bind(material_type)
        .bind(command.base_unit)
        .bind(command.default_zone)
        .bind(command.safety_stock)
        .bind(command.reorder_point)
        .bind(command.standard_price)
        .bind(command.map_price)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::parse_material(&row)
    }

    async fn update_material(
        &self,
        material_id: &str,
        command: UpdateMaterialCommand,
    ) -> AppResult<MaterialReadModel> {
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
            RETURNING
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
            return Err(AppError::business(
                "MATERIAL_NOT_FOUND",
                format!("物料不存在: {material_id}"),
            ));
        };

        Self::parse_material(&row)
    }

    async fn activate_material(&self, material_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            material_id,
            "MATERIAL_NOT_FOUND",
            format!("物料不存在: {material_id}"),
        )
    }

    async fn deactivate_material(&self, material_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            material_id,
            "MATERIAL_NOT_FOUND",
            format!("物料不存在: {material_id}"),
        )
    }
}

#[async_trait]
impl StorageBinRepository for PostgresMasterDataRepository {
    async fn list_bins(&self, query: MasterDataQuery) -> AppResult<Page<StorageBinReadModel>> {
        // 计划 §五.2:按区域查询货位 + 按是否可用查询货位 + 模糊搜索
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR bin_code ILIKE $3 OR zone ILIKE $3)
                AND ($4::text IS NULL OR zone = $4)
                AND ($5::text IS NULL OR status = $5)
            ORDER BY zone, bin_code
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.zone.as_deref())
        .bind(query.status.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_storage_bin)
    }

    async fn get_bin(&self, bin_code: &str) -> AppResult<StorageBinReadModel> {
        self.fetch_one_by_id(
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
            WHERE bin_code = $1
            "#,
            bin_code,
            "BIN_NOT_FOUND",
            format!("货位不存在: {bin_code}"),
            Self::parse_storage_bin,
        )
        .await
    }

    async fn create_bin(&self, command: CreateStorageBinCommand) -> AppResult<StorageBinReadModel> {
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
            RETURNING
                bin_code,
                zone,
                bin_type,
                capacity,
                current_occupied,
                status,
                notes,
                created_at,
                updated_at
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

        Self::parse_storage_bin(&row)
    }

    async fn update_bin(
        &self,
        bin_code: &str,
        command: UpdateStorageBinCommand,
    ) -> AppResult<StorageBinReadModel> {
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
            .ok_or_else(|| {
                AppError::business("BIN_NOT_FOUND", format!("货位不存在: {bin_code}"))
            })?;

            let bin_code_vo = BinCode::new(bin_code)?;
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
            // 触发 change_capacity 不变式:容量 > 0 且 >= current_occupied
            entity.change_capacity(new_capacity)?;
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
            RETURNING
                bin_code,
                zone,
                bin_type,
                capacity,
                current_occupied,
                status,
                notes,
                created_at,
                updated_at
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
            return Err(AppError::business(
                "BIN_NOT_FOUND",
                format!("货位不存在: {bin_code}"),
            ));
        };

        Self::parse_storage_bin(&row)
    }

    async fn activate_bin(&self, bin_code: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            bin_code,
            "BIN_NOT_FOUND",
            format!("货位不存在: {bin_code}"),
        )
    }

    async fn deactivate_bin(&self, bin_code: &str) -> AppResult<MutationAck> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_storage_bins
            SET status = '冻结', updated_at = NOW()
            WHERE bin_code = $1
            "#,
        )
        .bind(bin_code)
        .execute(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::affected_to_ack(
            result,
            bin_code,
            "BIN_NOT_FOUND",
            format!("货位不存在: {bin_code}"),
        )
    }

    async fn get_bin_capacity_utilization(
        &self,
        bin_code: &str,
    ) -> AppResult<BinCapacityUtilizationReadModel> {
        // 计划 §五.2:查询货位容量利用率。
        // utilization_pct 用 numeric 在 SQL 计算,避免浮点。
        let row = sqlx::query(
            r#"
            SELECT
                bin_code,
                zone,
                capacity,
                current_occupied,
                ROUND(current_occupied::numeric * 100 / capacity::numeric, 2) AS utilization_pct
            FROM mdm.mdm_storage_bins
            WHERE bin_code = $1
            "#,
        )
        .bind(bin_code)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "BIN_NOT_FOUND",
                format!("货位不存在: {bin_code}"),
            ));
        };

        Self::parse_bin_utilization(&row)
    }
}

#[async_trait]
impl SupplierRepository for PostgresMasterDataRepository {
    async fn list_suppliers(&self, query: MasterDataQuery) -> AppResult<Page<SupplierReadModel>> {
        // 计划 §五.3:模糊搜索 + 按 is_active 过滤
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR supplier_id ILIKE $3 OR supplier_name ILIKE $3)
                AND ($4::bool IS NULL OR is_active = $4)
            ORDER BY supplier_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_supplier)
    }

    async fn get_supplier(&self, supplier_id: &str) -> AppResult<SupplierReadModel> {
        self.fetch_one_by_id(
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
            WHERE supplier_id = $1
            "#,
            supplier_id,
            "SUPPLIER_NOT_FOUND",
            format!("供应商不存在: {supplier_id}"),
            Self::parse_supplier,
        )
        .await
    }

    async fn create_supplier(
        &self,
        command: CreateSupplierCommand,
    ) -> AppResult<SupplierReadModel> {
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
            RETURNING
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

        Self::parse_supplier(&row)
    }

    async fn update_supplier(
        &self,
        supplier_id: &str,
        command: UpdateSupplierCommand,
    ) -> AppResult<SupplierReadModel> {
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
            RETURNING
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
            return Err(AppError::business(
                "SUPPLIER_NOT_FOUND",
                format!("供应商不存在: {supplier_id}"),
            ));
        };

        Self::parse_supplier(&row)
    }

    async fn activate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            supplier_id,
            "SUPPLIER_NOT_FOUND",
            format!("供应商不存在: {supplier_id}"),
        )
    }

    async fn deactivate_supplier(&self, supplier_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            supplier_id,
            "SUPPLIER_NOT_FOUND",
            format!("供应商不存在: {supplier_id}"),
        )
    }
}

#[async_trait]
impl CustomerRepository for PostgresMasterDataRepository {
    async fn list_customers(&self, query: MasterDataQuery) -> AppResult<Page<CustomerReadModel>> {
        // 计划 §五.5:模糊搜索 + 按 is_active 过滤
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR customer_id ILIKE $3 OR customer_name ILIKE $3)
                AND ($4::bool IS NULL OR is_active = $4)
            ORDER BY customer_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_customer)
    }

    async fn get_customer(&self, customer_id: &str) -> AppResult<CustomerReadModel> {
        self.fetch_one_by_id(
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
            WHERE customer_id = $1
            "#,
            customer_id,
            "CUSTOMER_NOT_FOUND",
            format!("客户不存在: {customer_id}"),
            Self::parse_customer,
        )
        .await
    }

    async fn create_customer(
        &self,
        command: CreateCustomerCommand,
    ) -> AppResult<CustomerReadModel> {
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
            RETURNING
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

        Self::parse_customer(&row)
    }

    async fn update_customer(
        &self,
        customer_id: &str,
        command: UpdateCustomerCommand,
    ) -> AppResult<CustomerReadModel> {
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
            RETURNING
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
            return Err(AppError::business(
                "CUSTOMER_NOT_FOUND",
                format!("客户不存在: {customer_id}"),
            ));
        };

        Self::parse_customer(&row)
    }

    async fn activate_customer(&self, customer_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            customer_id,
            "CUSTOMER_NOT_FOUND",
            format!("客户不存在: {customer_id}"),
        )
    }

    async fn deactivate_customer(&self, customer_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            customer_id,
            "CUSTOMER_NOT_FOUND",
            format!("客户不存在: {customer_id}"),
        )
    }
}

#[async_trait]
impl MaterialSupplierRepository for PostgresMasterDataRepository {
    async fn list_material_suppliers(
        &self,
        material_id: &str,
    ) -> AppResult<Vec<MaterialSupplierReadModel>> {
        let material_exists: Option<bool> = sqlx::query_scalar(
            r#"
            SELECT TRUE
            FROM mdm.mdm_materials
            WHERE material_id = $1
            "#,
        )
        .bind(material_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        if material_exists.is_none() {
            return Err(AppError::business(
                "MATERIAL_NOT_FOUND",
                format!("物料不存在: {material_id}"),
            ));
        }

        let rows = sqlx::query(
            r#"
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
            ORDER BY ms.is_primary DESC, ms.supplier_id
            "#,
        )
        .bind(material_id)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        rows.iter()
            .map(Self::parse_material_supplier)
            .collect::<AppResult<Vec<_>>>()
    }

    async fn create_material_supplier(
        &self,
        command: CreateMaterialSupplierCommand,
    ) -> AppResult<MaterialSupplierReadModel> {
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
            WITH inserted AS (
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
                RETURNING *
            )
            SELECT
                inserted.id,
                inserted.material_id,
                inserted.supplier_id,
                s.supplier_name,
                inserted.is_primary,
                inserted.supplier_material_code,
                inserted.purchase_price,
                inserted.currency,
                inserted.lead_time_days,
                inserted.moq,
                inserted.quality_rating,
                inserted.is_active,
                inserted.created_at,
                inserted.updated_at
            FROM inserted
            LEFT JOIN mdm.mdm_suppliers s ON s.supplier_id = inserted.supplier_id
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

        Self::parse_material_supplier(&row)
    }

    async fn update_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
        command: UpdateMaterialSupplierCommand,
    ) -> AppResult<MaterialSupplierReadModel> {
        let row = sqlx::query(
            r#"
            WITH updated AS (
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
                RETURNING *
            )
            SELECT
                updated.id,
                updated.material_id,
                updated.supplier_id,
                s.supplier_name,
                updated.is_primary,
                updated.supplier_material_code,
                updated.purchase_price,
                updated.currency,
                updated.lead_time_days,
                updated.moq,
                updated.quality_rating,
                updated.is_active,
                updated.created_at,
                updated.updated_at
            FROM updated
            LEFT JOIN mdm.mdm_suppliers s ON s.supplier_id = updated.supplier_id
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

        Self::parse_material_supplier(&row)
    }

    async fn set_primary_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<MaterialSupplierReadModel> {
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
            WITH updated AS (
                UPDATE mdm.mdm_material_suppliers
                SET is_primary = TRUE, updated_at = NOW()
                WHERE material_id = $1 AND supplier_id = $2
                RETURNING *
            )
            SELECT
                updated.id,
                updated.material_id,
                updated.supplier_id,
                s.supplier_name,
                updated.is_primary,
                updated.supplier_material_code,
                updated.purchase_price,
                updated.currency,
                updated.lead_time_days,
                updated.moq,
                updated.quality_rating,
                updated.is_active,
                updated.created_at,
                updated.updated_at
            FROM updated
            LEFT JOIN mdm.mdm_suppliers s ON s.supplier_id = updated.supplier_id
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

        Self::parse_material_supplier(&row)
    }

    async fn remove_material_supplier(
        &self,
        material_id: &str,
        supplier_id: &str,
    ) -> AppResult<DeleteAck> {
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

        Ok(DeleteAck {
            resource_id: format!("{material_id}:{supplier_id}"),
            deleted: true,
        })
    }
}

#[async_trait]
impl ProductVariantRepository for PostgresMasterDataRepository {
    async fn list_variants(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<ProductVariantReadModel>> {
        // 计划 §五.6:支持模糊搜索 + is_active 过滤
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR variant_code ILIKE $3 OR variant_name ILIKE $3)
                AND ($4::bool IS NULL OR is_active = $4)
            ORDER BY variant_code
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_variant)
    }

    async fn get_variant(&self, variant_code: &str) -> AppResult<ProductVariantReadModel> {
        self.fetch_one_by_id(
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
            WHERE variant_code = $1
            "#,
            variant_code,
            "VARIANT_NOT_FOUND",
            format!("产品变体不存在: {variant_code}"),
            Self::parse_variant,
        )
        .await
    }

    async fn create_variant(
        &self,
        command: CreateProductVariantCommand,
    ) -> AppResult<ProductVariantReadModel> {
        // 计划 §五.6 领域规则:
        //   - 变体必须绑定有效成品物料(base_material_id 必须存在且 status='正常')
        //   - 绑定 BOM 必须存在且有效(bom_id 可选,如果给了必须 is_active=TRUE)
        // 用 FOR SHARE 锁住引用行,防 TOCTOU。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        // 1) base_material 必须存在、状态正常,且必须是成品物料。
        let material: Option<(String, String)> = sqlx::query_as(
            r#"
            SELECT status::text, material_type::text
            FROM mdm.mdm_materials
            WHERE material_id = $1
            FOR SHARE
            "#,
        )
        .bind(&command.base_material_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match material {
            None => {
                return Err(AppError::Validation(format!(
                    "base_material 不存在: {}",
                    command.base_material_id
                )));
            }
            Some((status, _)) if status != "正常" => {
                return Err(AppError::Validation(format!(
                    "base_material 不是正常状态,不能用于产品变体: {}",
                    command.base_material_id
                )));
            }
            Some((_, material_type)) if material_type != "成品" => {
                return Err(AppError::Validation(format!(
                    "base_material 必须是成品物料: {}",
                    command.base_material_id
                )));
            }
            Some(_) => {}
        }

        // 2) 如果指定了 bom_id,该 BOM 必须存在且 status='生效' 且 is_active=TRUE。
        if let Some(bom_id) = command.bom_id.as_deref() {
            let bom_state: Option<(String, bool)> = sqlx::query_as(
                r#"
                SELECT status::text, is_active
                FROM mdm.mdm_bom_headers
                WHERE bom_id = $1
                FOR SHARE
                "#,
            )
            .bind(bom_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
            match bom_state {
                None => {
                    return Err(AppError::Validation(format!("bom_id 不存在: {bom_id}")));
                }
                Some((status, is_active)) if status != "生效" || !is_active => {
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
            RETURNING
                variant_code,
                variant_name,
                base_material_id,
                bom_id,
                standard_cost,
                is_active,
                created_at,
                updated_at
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

        Self::parse_variant(&row)
    }

    async fn update_variant(
        &self,
        variant_code: &str,
        command: UpdateProductVariantCommand,
    ) -> AppResult<ProductVariantReadModel> {
        if let Some(bom_id) = command.bom_id.as_deref() {
            let bom_state: Option<(String, bool)> = sqlx::query_as(
                r#"
                SELECT status::text, is_active
                FROM mdm.mdm_bom_headers
                WHERE bom_id = $1
                "#,
            )
            .bind(bom_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
            match bom_state {
                None => return Err(AppError::Validation(format!("bom_id 不存在: {bom_id}"))),
                Some((status, is_active)) if status != "生效" || !is_active => {
                    return Err(AppError::Validation(format!(
                        "bom_id 未生效,不能绑定到产品变体: {bom_id}"
                    )));
                }
                Some(_) => {}
            }
        }

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
            RETURNING
                variant_code,
                variant_name,
                base_material_id,
                bom_id,
                standard_cost,
                is_active,
                created_at,
                updated_at
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
            return Err(AppError::business(
                "VARIANT_NOT_FOUND",
                format!("产品变体不存在: {variant_code}"),
            ));
        };

        Self::parse_variant(&row)
    }

    async fn activate_variant(&self, variant_code: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            variant_code,
            "VARIANT_NOT_FOUND",
            format!("产品变体不存在: {variant_code}"),
        )
    }

    async fn deactivate_variant(&self, variant_code: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            variant_code,
            "VARIANT_NOT_FOUND",
            format!("产品变体不存在: {variant_code}"),
        )
    }
}

#[async_trait]
impl BomRepository for PostgresMasterDataRepository {
    async fn list_boms(&self, query: MasterDataQuery) -> AppResult<Page<BomSummaryReadModel>> {
        // 计划 §五.7:模糊搜索 (bom_id / bom_name) + status + is_active 过滤
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));
        let status = Self::normalize_optional_bom_status(query.status.as_deref())?;

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                ($3::text IS NULL OR h.bom_id ILIKE $3 OR h.bom_name ILIKE $3)
                AND ($4::text IS NULL OR h.status = $4)
                AND ($5::bool IS NULL OR h.is_active = $5)
            GROUP BY
                h.bom_id,
                pm.material_name
            ORDER BY h.bom_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(status)
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_bom_summary)
    }

    async fn get_bom(&self, bom_id: &str) -> AppResult<BomDetailReadModel> {
        let header = self
            .fetch_one_by_id(
                r#"
            SELECT
                bom_id,
                bom_name,
                parent_material_id,
                variant_code,
                version,
                base_quantity,
                valid_from,
                valid_to,
                status,
                is_active,
                created_by,
                approved_by,
                approved_at,
                notes,
                created_at,
                updated_at
            FROM mdm.mdm_bom_headers
            WHERE bom_id = $1
            "#,
                bom_id,
                "BOM_NOT_FOUND",
                format!("BOM 不存在: {bom_id}"),
                Self::parse_bom_header,
            )
            .await?;

        let components = self.fetch_bom_components(bom_id).await?;

        Ok(BomDetailReadModel { header, components })
    }

    async fn create_bom(&self, command: CreateBomHeaderCommand) -> AppResult<BomHeaderReadModel> {
        let status = Self::normalize_optional_bom_status(command.status.as_deref())?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        let parent_status: Option<String> = sqlx::query_scalar(
            r#"
            SELECT status::text
            FROM mdm.mdm_materials
            WHERE material_id = $1
            FOR SHARE
            "#,
        )
        .bind(&command.parent_material_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;
        match parent_status {
            None => {
                return Err(AppError::Validation(format!(
                    "BOM 父物料不存在: {}",
                    command.parent_material_id
                )));
            }
            Some(status) if status == "停用" => {
                return Err(AppError::Validation(format!(
                    "BOM 父物料已停用: {}",
                    command.parent_material_id
                )));
            }
            _ => {}
        }

        if let Some(variant_code) = command.variant_code.as_deref() {
            let variant_active: Option<bool> = sqlx::query_scalar(
                r#"
                SELECT is_active
                FROM mdm.mdm_product_variants
                WHERE variant_code = $1
                FOR SHARE
                "#,
            )
            .bind(variant_code)
            .fetch_optional(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
            match variant_active {
                None => {
                    return Err(AppError::Validation(format!(
                        "产品变体不存在: {variant_code}"
                    )));
                }
                Some(false) => {
                    return Err(AppError::Validation(format!(
                        "产品变体已停用: {variant_code}"
                    )));
                }
                Some(true) => {}
            }
        }

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
            RETURNING
                bom_id,
                bom_name,
                parent_material_id,
                variant_code,
                version,
                base_quantity,
                valid_from,
                valid_to,
                status,
                is_active,
                created_by,
                approved_by,
                approved_at,
                notes,
                created_at,
                updated_at
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
        .bind(status)
        .bind(command.notes)
        .fetch_one(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        Self::parse_bom_header(&row)
    }

    async fn update_bom(
        &self,
        bom_id: &str,
        command: UpdateBomHeaderCommand,
    ) -> AppResult<BomHeaderReadModel> {
        let status = Self::normalize_optional_bom_status(command.status.as_deref())?;

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
            RETURNING
                bom_id,
                bom_name,
                parent_material_id,
                variant_code,
                version,
                base_quantity,
                valid_from,
                valid_to,
                status,
                is_active,
                created_by,
                approved_by,
                approved_at,
                notes,
                created_at,
                updated_at
            "#,
        )
        .bind(bom_id)
        .bind(command.bom_name)
        .bind(command.variant_code)
        .bind(command.version)
        .bind(command.base_quantity)
        .bind(command.valid_from)
        .bind(command.valid_to)
        .bind(status)
        .bind(command.is_active)
        .bind(command.notes)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "BOM_NOT_FOUND",
                format!("BOM 不存在: {bom_id}"),
            ));
        };

        Self::parse_bom_header(&row)
    }

    async fn list_components(&self, bom_id: &str) -> AppResult<Vec<BomComponentReadModel>> {
        self.ensure_bom_exists(bom_id).await?;
        self.fetch_bom_components(bom_id).await
    }

    async fn update_component(
        &self,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel> {
        let row = sqlx::query(
            r#"
            WITH updated AS (
                UPDATE mdm.mdm_bom_components
                SET
                    quantity = COALESCE($2, quantity),
                    unit = COALESCE($3, unit),
                    bom_level = COALESCE($4, bom_level),
                    scrap_rate = COALESCE($5, scrap_rate),
                    is_critical = COALESCE($6, is_critical)
                WHERE id = $1
                RETURNING *
            )
            SELECT
                updated.id,
                updated.bom_id,
                updated.parent_material_id,
                pm.material_name AS parent_material_name,
                updated.component_material_id,
                cm.material_name AS component_material_name,
                updated.quantity,
                updated.unit,
                updated.bom_level,
                updated.scrap_rate,
                updated.is_critical,
                updated.valid_from,
                updated.valid_to,
                updated.created_at
            FROM updated
            LEFT JOIN mdm.mdm_materials pm ON pm.material_id = updated.parent_material_id
            LEFT JOIN mdm.mdm_materials cm ON cm.material_id = updated.component_material_id
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
            return Err(AppError::business(
                "BOM_COMPONENT_NOT_FOUND",
                format!("BOM 组件不存在: {component_id}"),
            ));
        };

        Self::parse_bom_component(&row)
    }

    async fn update_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
        command: UpdateBomComponentCommand,
    ) -> AppResult<BomComponentReadModel> {
        self.ensure_bom_exists(bom_id).await?;

        let row = sqlx::query(
            r#"
            WITH updated AS (
                UPDATE mdm.mdm_bom_components
                SET
                    quantity = COALESCE($3, quantity),
                    unit = COALESCE($4, unit),
                    bom_level = COALESCE($5, bom_level),
                    scrap_rate = COALESCE($6, scrap_rate),
                    is_critical = COALESCE($7, is_critical)
                WHERE id = $1
                  AND bom_id = $2
                RETURNING *
            )
            SELECT
                updated.id,
                updated.bom_id,
                updated.parent_material_id,
                pm.material_name AS parent_material_name,
                updated.component_material_id,
                cm.material_name AS component_material_name,
                updated.quantity,
                updated.unit,
                updated.bom_level,
                updated.scrap_rate,
                updated.is_critical,
                updated.valid_from,
                updated.valid_to,
                updated.created_at
            FROM updated
            LEFT JOIN mdm.mdm_materials pm ON pm.material_id = updated.parent_material_id
            LEFT JOIN mdm.mdm_materials cm ON cm.material_id = updated.component_material_id
            "#,
        )
        .bind(component_id)
        .bind(bom_id)
        .bind(command.quantity)
        .bind(command.unit)
        .bind(command.bom_level)
        .bind(command.scrap_rate)
        .bind(command.is_critical)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "BOM_COMPONENT_NOT_FOUND",
                format!("BOM 组件不存在: bom_id={bom_id}, component_id={component_id}"),
            ));
        };

        Self::parse_bom_component(&row)
    }

    async fn remove_component(&self, component_id: i64) -> AppResult<DeleteAck> {
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
            return Err(AppError::business(
                "BOM_COMPONENT_NOT_FOUND",
                format!("BOM 组件不存在: {component_id}"),
            ));
        }

        Ok(DeleteAck {
            resource_id: component_id.to_string(),
            deleted: true,
        })
    }

    async fn remove_component_for_bom(
        &self,
        bom_id: &str,
        component_id: i64,
    ) -> AppResult<DeleteAck> {
        self.ensure_bom_exists(bom_id).await?;

        let result = sqlx::query(
            r#"
            DELETE FROM mdm.mdm_bom_components
            WHERE id = $1
              AND bom_id = $2
            "#,
        )
        .bind(component_id)
        .bind(bom_id)
        .execute(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        if result.rows_affected() == 0 {
            return Err(AppError::business(
                "BOM_COMPONENT_NOT_FOUND",
                format!("BOM 组件不存在: bom_id={bom_id}, component_id={component_id}"),
            ));
        }

        Ok(DeleteAck {
            resource_id: format!("{bom_id}:{component_id}"),
            deleted: true,
        })
    }

    async fn get_bom_tree(&self, bom_id: &str) -> AppResult<BomTreeReadModel> {
        let header = sqlx::query(
            r#"
            SELECT
                bom_id,
                bom_name,
                parent_material_id,
                variant_code,
                version,
                status,
                is_active
            FROM mdm.mdm_bom_headers
            WHERE bom_id = $1
            "#,
        )
        .bind(bom_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?
        .ok_or_else(|| AppError::business("BOM_NOT_FOUND", format!("BOM 不存在: {bom_id}")))?;

        let component_rows = sqlx::query(
            r#"
            SELECT
                c.id,
                c.component_material_id,
                cm.material_name AS component_material_name,
                c.quantity,
                c.unit,
                c.bom_level,
                c.scrap_rate,
                c.is_critical
            FROM mdm.mdm_bom_components c
            LEFT JOIN mdm.mdm_materials cm ON cm.material_id = c.component_material_id
            WHERE c.bom_id = $1
            ORDER BY c.bom_level, c.id
            "#,
        )
        .bind(bom_id)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let components = component_rows
            .iter()
            .map(Self::parse_bom_tree_component)
            .collect::<AppResult<Vec<_>>>()?;

        Ok(BomTreeReadModel {
            bom_id: Self::column(&header, "bom_id")?,
            bom_name: Self::column(&header, "bom_name")?,
            parent_material_id: Self::column(&header, "parent_material_id")?,
            variant_code: Self::column(&header, "variant_code")?,
            version: Self::column(&header, "version")?,
            status: Self::column(&header, "status")?,
            is_active: Self::column(&header, "is_active")?,
            components,
        })
    }

    async fn validate_bom(&self, bom_id: &str) -> AppResult<BomValidationReadModel> {
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
            SELECT
                $1::text AS bom_id,
                (SELECT ok FROM header_exists) AS header_exists,
                (SELECT cnt FROM component_count) AS component_count,
                (SELECT cnt FROM component_count) > 0 AS has_components,
                (SELECT cnt FROM self_reference) AS self_reference_count,
                (SELECT cnt FROM missing_components) AS missing_component_materials,
                EXISTS (SELECT 1 FROM cycle_walk) AS cycle_detected,
                (SELECT start_node FROM cycle_walk) AS cycle_node,
                    (SELECT ok FROM header_exists)
                    AND (SELECT cnt FROM component_count) > 0
                    AND (SELECT cnt FROM self_reference) = 0
                    AND (SELECT cnt FROM missing_components) = 0
                    AND NOT EXISTS (SELECT 1 FROM cycle_walk) AS valid
            "#,
        )
        .bind(bom_id)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::parse_validation(&row)
    }

    async fn preview_bom_explosion(
        &self,
        material_id: &str,
        quantity: i32,
        variant_code: Option<String>,
    ) -> AppResult<BomExplosionPreviewReadModel> {
        let rows = sqlx::query(
            r#"
            SELECT
                bom_level,
                parent_material_id,
                component_material_id,
                component_name,
                unit_qty,
                required_qty,
                available_qty,
                shortage_qty,
                is_critical
            FROM wms.fn_bom_explosion($1, $2, $3)
            ORDER BY bom_level, component_material_id
            "#,
        )
        .bind(material_id)
        .bind(quantity)
        .bind(variant_code.as_deref())
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let items = rows
            .iter()
            .map(Self::parse_explosion_item)
            .collect::<AppResult<Vec<_>>>()?;

        Ok(BomExplosionPreviewReadModel {
            material_id: material_id.to_string(),
            quantity,
            variant_code,
            items,
        })
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
            parent_material_id: MaterialId::new(header_row.get::<String, _>("parent_material_id"))?,
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
                parent_material_id: MaterialId::new(r.get::<String, _>("parent_material_id"))?,
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

        // 2) Diff components by component material. 当前 schema 唯一键是
        // (bom_id, component_material_id),所以同一 BOM 内组件物料只能出现一次。
        let existing: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT component_material_id
            FROM mdm.mdm_bom_components
            WHERE bom_id = $1
            "#,
        )
        .bind(bom.id().value())
        .fetch_all(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let existing_set: HashSet<String> = existing.into_iter().collect();
        let desired_set: HashSet<String> = bom
            .components()
            .iter()
            .map(|c| c.component_material_id.value().to_string())
            .collect();

        // 2a) DELETE 多余组件
        for child in existing_set.difference(&desired_set) {
            sqlx::query(
                r#"
                DELETE FROM mdm.mdm_bom_components
                WHERE bom_id = $1
                  AND component_material_id = $2
                "#,
            )
            .bind(bom.id().value())
            .bind(child)
            .execute(&mut *tx)
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;
        }

        // 2b) INSERT 新组件 / UPDATE 已有组件的可变属性
        for c in bom.components() {
            let parent = c.parent_material_id.value().to_string();
            let child = c.component_material_id.value().to_string();

            if existing_set.contains(&child) {
                sqlx::query(
                    r#"
                    UPDATE mdm.mdm_bom_components
                    SET parent_material_id = $3,
                        quantity           = $4,
                        unit               = $5,
                        bom_level          = $6,
                        scrap_rate         = $7,
                        is_critical        = $8
                    WHERE bom_id = $1
                      AND component_material_id = $2
                    "#,
                )
                .bind(bom.id().value())
                .bind(&child)
                .bind(&parent)
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
                         created_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NOW())
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
    async fn list_work_centers(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<WorkCenterReadModel>> {
        let (page, page_size, limit, offset) = Self::pagination(&query);

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
                work_center_id,
                work_center_name,
                location,
                capacity_per_day,
                efficiency,
                is_active,
                created_at
            FROM mdm.mdm_work_centers
            ORDER BY work_center_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_work_center)
    }

    async fn get_work_center(&self, work_center_id: &str) -> AppResult<WorkCenterReadModel> {
        self.fetch_one_by_id(
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
            WHERE work_center_id = $1
            "#,
            work_center_id,
            "WORK_CENTER_NOT_FOUND",
            format!("工作中心不存在: {work_center_id}"),
            Self::parse_work_center,
        )
        .await
    }

    async fn create_work_center(
        &self,
        command: CreateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel> {
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
            RETURNING
                work_center_id,
                work_center_name,
                location,
                capacity_per_day,
                efficiency,
                is_active,
                created_at
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

        Self::parse_work_center(&row)
    }

    async fn update_work_center(
        &self,
        work_center_id: &str,
        command: UpdateWorkCenterCommand,
    ) -> AppResult<WorkCenterReadModel> {
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
            RETURNING
                work_center_id,
                work_center_name,
                location,
                capacity_per_day,
                efficiency,
                is_active,
                created_at
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
            return Err(AppError::business(
                "WORK_CENTER_NOT_FOUND",
                format!("工作中心不存在: {work_center_id}"),
            ));
        };

        Self::parse_work_center(&row)
    }

    async fn activate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            work_center_id,
            "WORK_CENTER_NOT_FOUND",
            format!("工作中心不存在: {work_center_id}"),
        )
    }

    async fn deactivate_work_center(&self, work_center_id: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            work_center_id,
            "WORK_CENTER_NOT_FOUND",
            format!("工作中心不存在: {work_center_id}"),
        )
    }
}

#[async_trait]
impl QualityMasterRepository for PostgresMasterDataRepository {
    async fn list_inspection_chars(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<InspectionCharacteristicReadModel>> {
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));
        let material_type = Self::normalize_optional_material_type(query.material_type.as_deref())?;

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
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
                is_active,
                created_at
            FROM mdm.mdm_inspection_chars
            WHERE
                ($3::text IS NULL OR char_id ILIKE $3 OR char_name ILIKE $3)
                AND ($4::text IS NULL OR material_type::text = $4)
                AND ($5::bool IS NULL OR is_active = $5)
            ORDER BY char_id
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(material_type)
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_inspection_char)
    }

    async fn get_inspection_char(
        &self,
        char_id: &str,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        self.fetch_one_by_id(
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
                is_active,
                created_at
            FROM mdm.mdm_inspection_chars
            WHERE char_id = $1
            "#,
            char_id,
            "INSPECTION_CHAR_NOT_FOUND",
            format!("检验特性不存在: {char_id}"),
            Self::parse_inspection_char,
        )
        .await
    }

    async fn create_inspection_char(
        &self,
        command: CreateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        let material_type =
            Self::normalize_optional_material_type(command.material_type.as_deref())?;

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
                is_critical,
                is_active
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
                COALESCE($10, FALSE),
                TRUE
            )
            RETURNING
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
                is_active,
                created_at
            "#,
        )
        .bind(command.char_id)
        .bind(command.char_name)
        .bind(material_type)
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

        Self::parse_inspection_char(&row)
    }

    async fn update_inspection_char(
        &self,
        char_id: &str,
        command: UpdateInspectionCharCommand,
    ) -> AppResult<InspectionCharacteristicReadModel> {
        let material_type =
            Self::normalize_optional_material_type(command.material_type.as_deref())?;

        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        let current = sqlx::query(
            r#"
            SELECT char_name, lower_limit, upper_limit
            FROM mdm.mdm_inspection_chars
            WHERE char_id = $1
            FOR UPDATE
            "#,
        )
        .bind(char_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(current) = current else {
            return Err(AppError::business(
                "INSPECTION_CHAR_NOT_FOUND",
                format!("检验特性不存在: {char_id}"),
            ));
        };

        let lower_limit = command.lower_limit.or_else(|| {
            current
                .try_get::<Option<rust_decimal::Decimal>, _>("lower_limit")
                .ok()
                .flatten()
        });
        let upper_limit = command.upper_limit.or_else(|| {
            current
                .try_get::<Option<rust_decimal::Decimal>, _>("upper_limit")
                .ok()
                .flatten()
        });
        let mut entity = InspectionCharacteristic::new(
            InspectionCharId::new(char_id)?,
            current
                .try_get::<String, _>("char_name")
                .unwrap_or_else(|_| "Inspection".to_string()),
        )?;
        entity.set_limits(lower_limit, upper_limit)?;

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
            RETURNING
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
                is_active,
                created_at
            "#,
        )
        .bind(char_id)
        .bind(command.char_name)
        .bind(material_type)
        .bind(command.inspection_type)
        .bind(command.method)
        .bind(command.standard)
        .bind(command.unit)
        .bind(command.lower_limit)
        .bind(command.upper_limit)
        .bind(command.is_critical)
        .fetch_one(&mut *tx)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        tx.commit()
            .await
            .map_err(cuba_shared::map_master_data_db_error)?;

        Self::parse_inspection_char(&row)
    }

    async fn activate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_inspection_chars
            SET is_active = TRUE
            WHERE char_id = $1
            "#,
        )
        .bind(char_id)
        .execute(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::affected_to_ack(
            result,
            char_id,
            "INSPECTION_CHAR_NOT_FOUND",
            format!("检验特性不存在: {char_id}"),
        )
    }

    async fn deactivate_inspection_char(&self, char_id: &str) -> AppResult<MutationAck> {
        let result = sqlx::query(
            r#"
            UPDATE mdm.mdm_inspection_chars
            SET is_active = FALSE
            WHERE char_id = $1
            "#,
        )
        .bind(char_id)
        .execute(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::affected_to_ack(
            result,
            char_id,
            "INSPECTION_CHAR_NOT_FOUND",
            format!("检验特性不存在: {char_id}"),
        )
    }

    async fn list_defect_codes(
        &self,
        query: MasterDataQuery,
    ) -> AppResult<Page<DefectCodeReadModel>> {
        let (page, page_size, limit, offset) = Self::pagination(&query);
        let keyword_pattern = query.keyword.as_ref().map(|k| format!("%{}%", k));
        let severity = Self::normalize_optional_defect_severity(query.status.as_deref())?;

        let rows = sqlx::query(
            r#"
            SELECT
                COUNT(*) OVER() AS total,
                defect_code,
                defect_name,
                category,
                severity,
                description,
                is_active,
                created_at
            FROM mdm.mdm_defect_codes
            WHERE
                ($3::text IS NULL OR defect_code ILIKE $3 OR defect_name ILIKE $3)
                AND ($4::text IS NULL OR severity = $4)
                AND ($5::bool IS NULL OR is_active = $5)
            ORDER BY defect_code
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .bind(keyword_pattern.as_deref())
        .bind(severity)
        .bind(query.is_active)
        .fetch_all(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::page_from_rows(rows, page, page_size, Self::parse_defect_code)
    }

    async fn get_defect_code(&self, defect_code: &str) -> AppResult<DefectCodeReadModel> {
        self.fetch_one_by_id(
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
            WHERE defect_code = $1
            "#,
            defect_code,
            "DEFECT_CODE_NOT_FOUND",
            format!("不良代码不存在: {defect_code}"),
            Self::parse_defect_code,
        )
        .await
    }

    async fn create_defect_code(
        &self,
        command: CreateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel> {
        let severity = Self::normalize_defect_severity(&command.severity)?;

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
            RETURNING
                defect_code,
                defect_name,
                category,
                severity,
                description,
                is_active,
                created_at
            "#,
        )
        .bind(command.defect_code)
        .bind(command.defect_name)
        .bind(command.category)
        .bind(severity)
        .bind(command.description)
        .fetch_one(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        Self::parse_defect_code(&row)
    }

    async fn update_defect_code(
        &self,
        defect_code: &str,
        command: UpdateDefectCodeCommand,
    ) -> AppResult<DefectCodeReadModel> {
        let severity = Self::normalize_optional_defect_severity(command.severity.as_deref())?;

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
            RETURNING
                defect_code,
                defect_name,
                category,
                severity,
                description,
                is_active,
                created_at
            "#,
        )
        .bind(defect_code)
        .bind(command.defect_name)
        .bind(command.category)
        .bind(severity)
        .bind(command.description)
        .bind(command.is_active)
        .fetch_optional(&self.pool)
        .await
        .map_err(cuba_shared::map_master_data_db_error)?;

        let Some(row) = row else {
            return Err(AppError::business(
                "DEFECT_CODE_NOT_FOUND",
                format!("不良代码不存在: {defect_code}"),
            ));
        };

        Self::parse_defect_code(&row)
    }

    async fn activate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            defect_code,
            "DEFECT_CODE_NOT_FOUND",
            format!("不良代码不存在: {defect_code}"),
        )
    }

    async fn deactivate_defect_code(&self, defect_code: &str) -> AppResult<MutationAck> {
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

        Self::affected_to_ack(
            result,
            defect_code,
            "DEFECT_CODE_NOT_FOUND",
            format!("不良代码不存在: {defect_code}"),
        )
    }
}
