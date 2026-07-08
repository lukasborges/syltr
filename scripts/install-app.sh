#!/usr/bin/env bash
#
# Instala o Syltr no perfil do usuário (~/.local): binário, ícone, .desktop
# e metainfo. Não precisa de root.
#
#   ./scripts/install-app.sh              # instala em ~/.local
#   PREFIX=/usr/local sudo ./scripts/…    # instala no sistema
#   ./scripts/install-app.sh --uninstall  # remove
set -euo pipefail

APP_ID=dev.syltr.Syltr
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${PREFIX:-$HOME/.local}"

refresh_caches() {
    if command -v gtk4-update-icon-cache >/dev/null 2>&1; then
        gtk4-update-icon-cache -qtf "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    elif command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -qtf "$PREFIX/share/icons/hicolor" 2>/dev/null || true
    fi
    command -v update-desktop-database >/dev/null 2>&1 && \
        update-desktop-database "$PREFIX/share/applications" 2>/dev/null || true
}

if [[ "${1:-}" == "--uninstall" ]]; then
    rm -f "$PREFIX/bin/syltr" \
          "$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg" \
          "$PREFIX/share/applications/$APP_ID.desktop" \
          "$PREFIX/share/metainfo/$APP_ID.metainfo.xml" \
          "$HOME/.local/share/locale/"*/LC_MESSAGES/syltr.mo
    refresh_caches
    echo "✓ Syltr removido de $PREFIX."
    exit 0
fi

cd "$ROOT"

echo "==> Compilando (release)…"
cargo build --release

echo "==> Instalando binário em $PREFIX/bin…"
install -Dm755 "target/release/syltr" "$PREFIX/bin/syltr"

echo "==> Instalando ícone…"
install -Dm644 "data/icons/$APP_ID.svg" \
    "$PREFIX/share/icons/hicolor/scalable/apps/$APP_ID.svg"

echo "==> Instalando .desktop (Exec absoluto)…"
install -d "$PREFIX/share/applications"
sed "s|^Exec=.*|Exec=$PREFIX/bin/syltr|" "data/$APP_ID.desktop" \
    > "$PREFIX/share/applications/$APP_ID.desktop"
chmod 644 "$PREFIX/share/applications/$APP_ID.desktop"

echo "==> Instalando metainfo…"
install -Dm644 "data/$APP_ID.metainfo.xml" \
    "$PREFIX/share/metainfo/$APP_ID.metainfo.xml"

echo "==> Compilando traduções…"
# O app faz bindtextdomain em ~/.local/share (user_data_dir)/locale. Instala
# por idioma base (ex.: 'pt'); o gettext faz fallback de pt_BR/pt_PT -> pt.
LOCALE_DIR="$HOME/.local/share/locale"
if command -v msgfmt >/dev/null 2>&1; then
    for po in po/*.po; do
        [[ -e "$po" ]] || continue
        lang="$(basename "$po" .po)"
        install -d "$LOCALE_DIR/$lang/LC_MESSAGES"
        msgfmt "$po" -o "$LOCALE_DIR/$lang/LC_MESSAGES/syltr.mo"
    done
    echo "   traduções instaladas em $LOCALE_DIR"
else
    echo "   ! msgfmt não encontrado (pacote 'gettext'); traduções não instaladas."
fi

echo "==> Atualizando caches…"
refresh_caches

echo
echo "✓ Instalado em $PREFIX."
if [[ ":$PATH:" != *":$PREFIX/bin:"* ]]; then
    echo "  (Dica: adicione $PREFIX/bin ao PATH para rodar 'syltr' no terminal.)"
fi
echo "  Procure 'Syltr' no menu de aplicativos do GNOME."
