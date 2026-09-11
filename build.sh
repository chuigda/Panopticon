#!/usr/bin/env bash

cd frontend
npm install
if [ $? -ne 0 ]; then
  echo "npm install failed"
  exit 1
fi

npm run build
if [ $? -ne 0 ]; then
  echo "npm run build failed"
  exit 1
fi

cd ../backstage
cargo build --release
if [ $? -ne 0 ]; then
  echo "cargo build failed"
  exit 1
fi

cd ..
echo "Build succeeded. Target: backstage/target/release/panopticon."
