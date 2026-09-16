#!/usr/bin/env bash
# 遍历 example/ 下的 .toml，生成 backstage/src/examples.rs (静态路由) 和 examples.json (目录树)
set -euo pipefail

cd "$(dirname "$0")"

SRC_DIR="example"
OUT_RS="backstage/src/examples.rs"
OUT_JSON="backstage/src/examples.json"
URL_PREFIX="/examples"
# include_str! 相对于 backstage/src/
INCLUDE_PREFIX="../../$SRC_DIR"

# 递归生成 JSON 对象：目录 -> 对象，文件 -> URL 字符串
emit_dir() {
  local dir="$1" rel="$2" indent="$3"
  local items=()
  local entry name child

  for entry in "$dir"/*; do
    [ -e "$entry" ] || continue
    name="$(basename "$entry")"
    if [ -d "$entry" ]; then
      child="$(emit_dir "$entry" "${rel:+$rel/}$name" "$indent  ")"
      items+=("$indent  \"$name\": $child")
    elif [[ "$name" == *.toml ]]; then
      local relpath="${rel:+$rel/}$name"
      items+=("$indent  \"$name\": \"$URL_PREFIX/$relpath\"")
    fi
  done

  if [ ${#items[@]} -eq 0 ]; then
    echo "{}"
    return
  fi

  echo "{"
  local i
  for i in "${!items[@]}"; do
    if [ "$i" -lt $((${#items[@]} - 1)) ]; then
      echo "${items[$i]},"
    else
      echo "${items[$i]}"
    fi
  done
  echo "$indent}"
}

emit_dir "$SRC_DIR" "" "" > "$OUT_JSON"

mapfile -t routes < <(cd "$SRC_DIR" && find . -type f -name '*.toml' | sed 's|^\./||' | sort)

{
  echo "// 由 gen-examples.sh 自动生成，请勿手动修改"
  echo ""
  echo "fn examples_router<S: Clone + Send + Sync + 'static>() -> Router<S> {"
  echo "    Router::new()"
  for relpath in "${routes[@]}"; do
    echo "        .route("
    echo "            \"$URL_PREFIX/$relpath\","
    echo "            get(|| async { toml_response(include_str!(\"$INCLUDE_PREFIX/$relpath\")) }),"
    echo "        )"
  done
  echo "}"
  echo ""
  echo "fn toml_response(body: &'static str) -> impl axum::response::IntoResponse {"
  echo "    ([(header::CONTENT_TYPE, \"application/toml\")], body)"
  echo "}"
} > "$OUT_RS"

echo "generated ${#routes[@]} routes -> $OUT_RS, $OUT_JSON"
