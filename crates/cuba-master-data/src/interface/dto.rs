use serde::Deserialize;

pub use crate::application::{
    CopyBomCommand, CreateBomComponentCommand, CreateBomHeaderCommand, CreateCustomerCommand,
    CreateDefectCodeCommand, CreateInspectionCharCommand, CreateMaterialCommand,
    CreateMaterialSupplierCommand, CreateProductVariantCommand, CreateStorageBinCommand,
    CreateSupplierCommand, CreateWorkCenterCommand, MasterDataQuery, UpdateBomComponentCommand,
    UpdateBomHeaderCommand, UpdateCustomerCommand, UpdateDefectCodeCommand,
    UpdateInspectionCharCommand, UpdateMaterialCommand, UpdateMaterialSupplierCommand,
    UpdateProductVariantCommand, UpdateStorageBinCommand, UpdateSupplierCommand,
    UpdateWorkCenterCommand,
};

#[derive(Debug, Clone, Deserialize)]
pub struct BomExplosionPreviewQuery {
    pub material_id: String,
    pub quantity: i32,
    pub variant_code: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BomExplosionPreviewRequest {
    pub quantity: i32,
    pub variant_code: Option<String>,
}
