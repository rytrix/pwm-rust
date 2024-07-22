#!/bin/bash
# $1 first version
# $2 second version
sed -i "s/version = \"$1\"/version = \"$2\"/g" pwm-gui/Cargo.toml
sed -i "s/version = \"$1\"/version = \"$2\"/g" pwm-cli/Cargo.toml
sed -i "s/version = \"$1\"/version = \"$2\"/g" pwm-lib/Cargo.toml
sed -i "s/version = \"$1\"/version = \"$2\"/g" pwm-proc/Cargo.toml
sed -i "s/version = \"$1\"/version = \"$2\"/g" pwm-db/Cargo.toml
