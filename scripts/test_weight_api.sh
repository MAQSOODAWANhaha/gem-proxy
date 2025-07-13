#!/bin/bash

# 权重管理 API 测试脚本

API_BASE="http://localhost:9090"

echo "🚀 开始测试权重管理 API..."

# 1. 测试获取权重统计
echo ""
echo "📊 测试权重统计 API: GET /api/weights/stats"
curl -s "$API_BASE/api/weights/stats" | jq . || echo "请求失败或服务未启动"

# 2. 测试获取权重分配
echo ""
echo "📈 测试权重分配 API: GET /api/weights/distribution"
curl -s "$API_BASE/api/weights/distribution" | jq . || echo "请求失败"

# 3. 测试权重优化建议
echo ""
echo "🎯 测试权重优化建议 API: GET /api/weights/optimize"
curl -s "$API_BASE/api/weights/optimize" | jq . || echo "请求失败"

# 4. 测试更新权重 (假设存在 key1)
echo ""
echo "⚖️ 测试更新权重 API: PUT /api/weights/key1"
curl -s -X PUT "$API_BASE/api/weights/key1" \
  -H "Content-Type: application/json" \
  -d '{"weight": 150}' | jq . || echo "请求失败"

# 5. 测试批量更新权重
echo ""
echo "📦 测试批量更新权重 API: POST /api/weights/batch"
curl -s -X POST "$API_BASE/api/weights/batch" \
  -H "Content-Type: application/json" \
  -d '{
    "updates": [
      {"key_id": "key1", "weight": 200},
      {"key_id": "key2", "weight": 100}
    ]
  }' | jq . || echo "请求失败"

# 6. 测试智能重平衡
echo ""
echo "🔄 测试智能重平衡 API: POST /api/weights/rebalance"
curl -s -X POST "$API_BASE/api/weights/rebalance" | jq . || echo "请求失败"

echo ""
echo "✅ 权重管理 API 测试完成！"