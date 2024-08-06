#!/usr/bin/env bash

cargo build --release
mkdir -p dist/
cp target/release/pwm-gui dist/pwm-gui
cp target/release/pwm-cli dist/pwm-cli
cp extra/pwm.png dist/pwm.png

cat <<EOF > dist/install.sh
#!/usr/bin/env bash

if [ -z \$1 ]; then
    echo "No target provided"
    exit 0
fi

if [ -d \$1 ]; then
    if [ ! -d \$1/bin ]; then
        mkdir -p \$1/bin
    fi
    cp pwm-gui pwm-cli \$1/bin

    if [ ! -d \$1/share/applications ]; then
        mkdir -p \$1/share/applications
    fi
    cat <<EOF1 > \$1/share/applications/pwm.desktop
[Desktop Entry]
Type=Application
Terminal=false
Exec=\$1/bin/pwm-gui
TryExec=\$1/bin/pwm-gui
Name=Pwm
Icon=\$1/share/icons/pwm.png
Categories=Security;Utility
WM_CLASS=PWM
EOF1

    if [ ! -d \$1/share/icons ]; then
        mkdir -p \$1/share/icons
    fi
    cp pwm.png \$1/share/icons
else
    echo "Directory \"\$1\" does not exist, not installing"
fi
EOF

chmod +x dist/install.sh