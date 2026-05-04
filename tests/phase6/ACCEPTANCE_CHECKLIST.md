# Phase 6 验收清单

## 验收标准 (10项必须全部通过)

### 1. ✅ BOM 爆炸预览

**命令:**
```bash
curl -X POST http://localhost:8080/api/production/bom-explosion \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "quantity": 10,
    "merge_components": true
  }'
```

**期望结果:**
- `success = true`
- `data.components` 不为空
- 每个组件有 `required_qty`, `available_qty`, `shortage_qty`

**验证:**
- [ ] API 返回成功
- [ ] 组件列表不为空
- [ ] 组件数据完整

---

### 2. ✅ 创建生产订单

**命令:**
```bash
curl -X POST http://localhost:8080/api/production-orders \
  -H "Content-Type: application/json" \
  -d '{
    "variant_code": "FIN-A001",
    "finished_material_id": "FIN001",
    "bom_id": "BOM-FIN-A01",
    "planned_qty": 10,
    "work_center_id": "WC-ASSY-01",
    "planned_start_date": "2026-05-05",
    "planned_end_date": "2026-05-08",
    "remark": "phase 6 production order"
  }'
```

**期望结果:**
- `success = true`
- `data.order_id` 存在 (格式: `MO-xxxxxxxx`)
- `data.status = "PLANNED"`
- `data.component_count > 0`

**验证:**
- [ ] 订单创建成功
- [ ] 返回有效的 order_id
- [ ] 状态为 PLANNED
- [ ] 组件数量大于 0

**保存 ORDER_ID:**
```bash
export ORDER_ID="<从响应中获取的 order_id>"
```

---

### 3. ✅ 自动生成组件行

**命令:**
```bash
curl http://localhost:8080/api/production-orders/$ORDER_ID/components
```

**期望结果:**
- `success = true`
- `data` 是组件行数组
- 每个组件有 `material_id`, `planned_qty`, `actual_qty`

**验证:**
- [ ] 组件行自动生成
- [ ] 组件数据完整
- [ ] planned_qty 正确

---

### 4. ✅ 下达生产订单

**命令:**
```bash
curl -X POST http://localhost:8080/api/production-orders/$ORDER_ID/release \
  -H "Content-Type: application/json" \
  -d '{
    "remark": "release phase 6 production order"
  }'
```

**期望结果:**
- `success = true`
- `data.status = "RELEASED"`

**验证:**
- [ ] 订单下达成功
- [ ] 状态变更为 RELEASED

---

### 5. ✅ 一键完工

**命令:**
```bash
curl -X POST http://localhost:8080/api/production-orders/$ORDER_ID/complete \
  -H "Content-Type: application/json" \
  -d '{
    "completed_qty": 10,
    "finished_batch_number": "BATCH-FIN001-P6-001",
    "finished_to_bin": "FG-A01",
    "posting_date": "2026-05-04T14:00:00Z",
    "pick_strategy": "FEFO",
    "remark": "phase 6 production complete"
  }'
```

**期望结果:**
- `success = true`
- `data.status = "COMPLETED"`
- `data.completed_qty = 10`
- `data.reports_stale = true`

**验证:**
- [ ] 完工成功
- [ ] 状态变更为 COMPLETED
- [ ] 完工数量正确
- [ ] reports_stale 标志设置

---

### 6. ✅ 组件 261 领料事务

**从完工响应中验证:**
```json
{
  "data": {
    "component_transactions": [
      {
        "movement_type": "261",
        "material_id": "...",
        "quantity": -X,
        "transaction_id": "..."
      }
    ]
  }
}
```

**期望结果:**
- `component_transactions` 数组不为空
- 每个事务 `movement_type = "261"`
- `quantity` 为负数 (消耗)

**验证:**
- [ ] 261 事务存在
- [ ] 事务数量正确
- [ ] transaction_id 有效

**数据库验证:**
```sql
SELECT transaction_id, movement_type, material_id, quantity, batch_number
FROM wms.wms_transactions
WHERE reference_doc = '$ORDER_ID'
  AND movement_type = '261'
ORDER BY transaction_date DESC;
```

**验证:**
- [ ] 数据库中有 261 记录
- [ ] 数量为负数
- [ ] 批次号正确

---

### 7. ✅ 成品 101 入库事务

**从完工响应中验证:**
```json
{
  "data": {
    "finished_transaction": {
      "movement_type": "101",
      "material_id": "FIN001",
      "quantity": 10,
      "batch_number": "BATCH-FIN001-P6-001",
      "transaction_id": "..."
    }
  }
}
```

**期望结果:**
- `finished_transaction` 存在
- `movement_type = "101"`
- `quantity` 为正数 (入库)
- `batch_number = "BATCH-FIN001-P6-001"`

**验证:**
- [ ] 101 事务存在
- [ ] 事务数量正确
- [ ] 批次号正确

**数据库验证:**
```sql
SELECT transaction_id, movement_type, material_id, quantity, batch_number, to_bin
FROM wms.wms_transactions
WHERE reference_doc = '$ORDER_ID'
  AND movement_type = '101'
ORDER BY transaction_date DESC;
```

**验证:**
- [ ] 数据库中有 101 记录
- [ ] 数量为正数
- [ ] 目标货位正确

---

### 8. ✅ 批次谱系记录

**命令:**
```bash
curl http://localhost:8080/api/production-orders/$ORDER_ID/genealogy
```

**期望结果:**
- `success = true`
- `data` 数组不为空
- 每条记录有:
  - `parent_batch_number` (成品批次)
  - `component_batch_number` (组件批次)
  - `consumed_qty` (消耗数量)
  - `output_qty` (产出数量)

**验证:**
- [ ] 谱系记录存在
- [ ] 成品批次正确
- [ ] 组件批次正确
- [ ] 数量关系正确

**数据库验证:**
```sql
SELECT
    production_order_id,
    parent_batch_number,
    component_batch_number,
    parent_material_id,
    component_material_id,
    consumed_qty,
    output_qty
FROM wms.wms_batch_genealogy
WHERE production_order_id = '$ORDER_ID'
ORDER BY created_at;
```

**验证:**
- [ ] wms_batch_genealogy 有记录
- [ ] production_order_id 正确
- [ ] 批次关联正确

---

### 9. ✅ 成本差异记录

**命令:**
```bash
curl http://localhost:8080/api/production-orders/$ORDER_ID/variance
```

**期望结果:**
- `success = true`
- `data.planned_material_cost` 存在
- `data.actual_material_cost` 存在
- `data.material_variance` 可计算
- `data.total_variance` 可计算

**验证:**
- [ ] 差异记录存在
- [ ] 计划成本有值
- [ ] 实际成本有值
- [ ] 差异计算正确

**数据库验证:**
```sql
SELECT
    variance_id,
    order_id,
    output_material_id,
    planned_material_cost,
    actual_material_cost,
    material_variance,
    planned_labor_cost,
    actual_labor_cost,
    labor_variance,
    total_variance
FROM wms.wms_production_variances
WHERE order_id = '$ORDER_ID';
```

**验证:**
- [ ] wms_production_variances 有记录
- [ ] order_id 正确
- [ ] 成本数据完整

---

### 10. ✅ 库存变化验证

**步骤 1: 刷新报表**
```bash
curl -X POST http://localhost:8080/api/reports/refresh
```

**期望结果:**
- `success = true`
- `data.refreshed = true`

**验证:**
- [ ] 报表刷新成功

**步骤 2: 查询成品库存**
```bash
curl "http://localhost:8080/api/reports/current-stock?material_id=FIN001"
```

**期望结果:**
- 成品 FIN001 库存增加
- 批次 BATCH-FIN001-P6-001 存在

**验证:**
- [ ] 成品库存增加
- [ ] 新批次可查询

**步骤 3: 查询组件库存**
```bash
curl "http://localhost:8080/api/reports/current-stock?material_id=<组件物料号>"
```

**期望结果:**
- 组件库存减少
- 对应批次数量减少

**验证:**
- [ ] 组件库存减少
- [ ] 批次数量正确

**数据库验证:**
```sql
-- 检查成品库存
SELECT material_id, current_stock, map_price
FROM mdm.mdm_materials
WHERE material_id = 'FIN001';

-- 检查成品批次
SELECT batch_number, material_id, current_stock, current_bin, quality_status
FROM wms.wms_batches
WHERE batch_number = 'BATCH-FIN001-P6-001';

-- 检查成品货位库存
SELECT material_id, bin_code, batch_number, qty, quality_status
FROM wms.wms_bin_stock
WHERE batch_number = 'BATCH-FIN001-P6-001';

-- 检查组件库存变化
SELECT
    t.material_id,
    m.material_name,
    SUM(CASE WHEN t.movement_type = '261' THEN t.quantity ELSE 0 END) AS consumed_qty,
    m.current_stock
FROM wms.wms_transactions t
JOIN mdm.mdm_materials m ON m.material_id = t.material_id
WHERE t.reference_doc = '$ORDER_ID'
  AND t.movement_type = '261'
GROUP BY t.material_id, m.material_name, m.current_stock;
```

**验证:**
- [ ] 成品物料主数据更新
- [ ] 成品批次创建
- [ ] 成品货位库存增加
- [ ] 组件库存减少

---

## 总结

### 必须全部通过的验收项 (10/10)

- [ ] 1. BOM 爆炸预览成功
- [ ] 2. 创建生产订单成功
- [ ] 3. 自动生成组件行
- [ ] 4. 下达订单成功
- [ ] 5. 一键完工成功
- [ ] 6. 产生 261 领料事务
- [ ] 7. 产生 101 入库事务
- [ ] 8. wms_batch_genealogy 有记录
- [ ] 9. wms_production_variances 有记录
- [ ] 10. 库存变化正确

### 快速验收命令

运行完整验收脚本：
```bash
BASE_URL=http://localhost:8080 ./tests/phase6/phase6_acceptance.sh
```

### 验收通过标准

**所有 10 项必须通过，且:**
- API 响应正确
- 数据库记录完整
- 库存变化准确
- 批次谱系正确
- 成本差异计算

### 验收失败处理

如果任何一项失败：
1. 检查数据库主数据是否完整
2. 检查 API 服务日志
3. 检查数据库函数是否存在
4. 重新运行单项测试
5. 查看 `PHASE6_STATUS.md` 排查问题

---

**验收负责人:** _____________
**验收日期:** _____________
**验收结果:** ⬜ 通过 ⬜ 不通过
**备注:** _____________
