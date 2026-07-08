#!/usr/bin/env bash
#
# Syltr — instalador de dependências de desenvolvimento (Arch Linux)
# =================================================================
# Agregador de serviços de mensagens em GTK4 + libadwaita
# + Rust, usando a engine do Chromium via CEF (Chromium Embedded Framework).
#
# Uso:
#   ./scripts/install-deps.sh            # instala tudo
#   ./scripts/install-deps.sh --no-cef   # pula o CEF (usa só WebKitGTK p/ dev)
#   ./scripts/install-deps.sh --no-ide   # pula GNOME Builder e ferramentas de IDE
#
# Idempotente: pode rodar quantas vezes quiser.
set -euo pipefail

# ----------------------------------------------------------------------------
# Cores / helpers
# ----------------------------------------------------------------------------
if [[ -t 1 ]]; then
    C_RESET=$'\e[0m'; C_BOLD=$'\e[1m'; C_BLUE=$'\e[34m'; C_GREEN=$'\e[32m'
    C_YELLOW=$'\e[33m'; C_RED=$'\e[31m'
else
    C_RESET=; C_BOLD=; C_BLUE=; C_GREEN=; C_YELLOW=; C_RED=
fi
info()  { printf '%s==>%s %s\n' "$C_BLUE$C_BOLD" "$C_RESET" "$*"; }
ok()    { printf '%s  ✓%s %s\n' "$C_GREEN" "$C_RESET" "$*"; }
warn()  { printf '%s  !%s %s\n' "$C_YELLOW" "$C_RESET" "$*"; }
err()   { printf '%s  ✗%s %s\n' "$C_RED" "$C_RESET" "$*" >&2; }

INSTALL_CEF=1
INSTALL_IDE=1
for arg in "$@"; do
    case "$arg" in
        --no-cef) INSTALL_CEF=0 ;;
        --no-ide) INSTALL_IDE=0 ;;
        -h|--help) grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) err "argumento desconhecido: $arg"; exit 1 ;;
    esac
done

# ----------------------------------------------------------------------------
# Pré-condições
# ----------------------------------------------------------------------------
if [[ $EUID -eq 0 ]]; then
    err "Não rode como root. O script chama 'sudo' quando necessário."
    exit 1
fi
if ! command -v pacman >/dev/null 2>&1; then
    err "Este script é para Arch Linux (pacman não encontrado)."
    exit 1
fi

# ----------------------------------------------------------------------------
# Pacotes dos repositórios oficiais
# ----------------------------------------------------------------------------
# Toolchain / build
PKGS_BASE=(
    base-devel        # gcc, make, etc. (necessário p/ compilar crates -sys)
    git
    pkgconf           # pkg-config, usado pelos crates -sys do GTK
    cmake ninja meson # sistemas de build (GNOME usa meson)
    clang lld         # linker rápido; alguns -sys pedem clang
    gettext           # msgfmt + libintl (i18n da interface)
)

# Stack GNOME / GTK (headers + runtime; no Arch vêm no mesmo pacote)
PKGS_GNOME=(
    gtk4
    libadwaita
    glib2
    gobject-introspection
    graphene
    cairo pango gdk-pixbuf2
    librsvg           # ícones SVG symbolic
    blueprint-compiler # .blp -> .ui (padrão moderno de UI do GNOME)
    desktop-file-utils
    appstream appstream-glib   # validação de metainfo.xml
    hicolor-icon-theme adwaita-icon-theme
)

# WebKitGTB — engine WebKit. Fica como FALLBACK de desenvolvimento
# (a engine final do Syltr é Chromium via CEF, veja abaixo).
PKGS_WEBKIT=(
    webkitgtk-6.0
)

# Verificação ortográfica: enchant + backend hunspell + dicionário. O WebKit
# usa o enchant, que precisa da lib 'hunspell' (libhunspell-1.7.so) e de ao
# menos um dicionário. pt_BR não está nos repos oficiais (AUR: hunspell-pt-br).
PKGS_SPELL=(
    enchant
    hunspell
    hunspell-en_us
)

# Ferramentas de IDE / dev GNOME
PKGS_IDE=(
    gnome-builder     # IDE oficial do GNOME
    devhelp           # navegador de documentação de API
    d-spy             # inspetor de D-Bus
    sysprof           # profiler
    # (os demos 'gtk4-widget-factory'/'gtk4-demo' já vêm no pacote 'gtk4')
)

info "Atualizando índice de pacotes e instalando toolchain + stack GNOME..."
sudo pacman -Syu --needed --noconfirm \
    "${PKGS_BASE[@]}" "${PKGS_GNOME[@]}" "${PKGS_WEBKIT[@]}" "${PKGS_SPELL[@]}"
ok "Toolchain, GTK4/libadwaita, WebKitGTK e verificação ortográfica instalados."
warn "Dicionário pt_BR não está nos repos oficiais — via AUR: paru -S hunspell-pt-br"

if [[ $INSTALL_IDE -eq 1 ]]; then
    info "Instalando IDE e ferramentas de desenvolvimento GNOME..."
    sudo pacman -S --needed --noconfirm "${PKGS_IDE[@]}"
    ok "GNOME Builder + Devhelp + D-Spy + Sysprof instalados."
else
    warn "Pulando IDE (--no-ide)."
fi

# ----------------------------------------------------------------------------
# Rust (via rustup — recomendado para dev)
# ----------------------------------------------------------------------------
info "Configurando toolchain Rust..."
if command -v cargo >/dev/null 2>&1 && command -v rustc >/dev/null 2>&1; then
    # Já existe um Rust (pacote 'rust' do Arch ou rustup). Não mexemos —
    # 'rust' e 'rustup' conflitam; trocar exigiria remover um deles.
    ok "Rust já instalado: $(rustc --version | awk '{print $2}') ($(cargo --version | awk '{print $2}'))."
    if command -v rustup >/dev/null 2>&1; then
        rustup component add rust-analyzer rust-src clippy rustfmt 2>/dev/null || true
    else
        warn "Usando o pacote 'rust' do Arch. Ele já inclui clippy/rustfmt/rust-analyzer."
        warn "Se preferir gerenciar versões com rustup: sudo pacman -Rns rust && sudo pacman -S rustup"
    fi
else
    # Nenhum Rust presente: instala rustup (gerencia toolchains).
    sudo pacman -S --needed --noconfirm rustup
    rustup default stable
    rustup component add rust-analyzer rust-src clippy rustfmt
    ok "Rust $(rustc --version | awk '{print $2}') instalado via rustup."
fi

# ----------------------------------------------------------------------------
# CEF (Chromium Embedded Framework) — a engine Chromium
# ----------------------------------------------------------------------------
# CEF não está nos repos oficiais; vem do AUR. Precisa de um AUR helper.
detect_aur_helper() {
    for h in paru yay; do
        if command -v "$h" >/dev/null 2>&1; then echo "$h"; return 0; fi
    done
    return 1
}

cef_via_binary_hint() {
    cat <<'EOF'

    O jeito mais confiável de obter o CEF (engine Chromium) é o binário oficial
    pré-compilado (Spotify CEF builds) — evita horas de build do Chromium:

      1. Abra:  https://cef-builds.spotifycdn.com/index.html
      2. Baixe o "Minimal Distribution" para Linux 64-bit.
      3. Extraia e exporte a variável que o crate 'cef' usa:

           export CEF_PATH=/caminho/para/cef_binary_XXXX_linux64_minimal
           # adicione essa linha ao seu ~/.bashrc para persistir

    Só será necessário quando migrarmos a engine de WebKitGTK para CEF.
EOF
}

if [[ $INSTALL_CEF -eq 1 ]]; then
    info "Preparando CEF (engine Chromium)..."
    if AUR=$(detect_aur_helper); then
        # Nomes do CEF no AUR variam com o tempo; tenta os candidatos conhecidos.
        cef_ok=0
        for pkg in cef-minimal cef; do
            if "$AUR" -S --needed "$pkg" 2>/dev/null; then
                ok "CEF instalado via $AUR ($pkg)."
                cef_ok=1
                break
            fi
        done
        if [[ $cef_ok -eq 0 ]]; then
            warn "Nenhum pacote CEF encontrado no AUR (o 'cef' completo compila o"
            warn "Chromium do zero e leva horas). Recomendo o binário pré-compilado:"
            cef_via_binary_hint
        fi
    else
        warn "Nenhum AUR helper (paru/yay) encontrado."
        cef_via_binary_hint
    fi
else
    warn "Pulando CEF (--no-cef). Desenvolvimento segue com WebKitGTK."
fi

# ----------------------------------------------------------------------------
# Resumo
# ----------------------------------------------------------------------------
echo
info "Verificação rápida:"
for lib in gtk4 libadwaita-1 webkitgtk-6.0; do
    if v=$(pkg-config --modversion "$lib" 2>/dev/null); then
        ok "$lib $v"
    else
        err "$lib NÃO encontrado"
    fi
done
command -v gnome-builder >/dev/null 2>&1 && ok "gnome-builder $(gnome-builder --version 2>/dev/null | head -1)"
command -v cargo >/dev/null 2>&1 && ok "cargo $(cargo --version | awk '{print $2}')"
if pacman -Qq cef-minimal >/dev/null 2>&1; then ok "cef-minimal instalado"; fi

echo
ok "Ambiente de desenvolvimento pronto."
echo "   Próximo passo:  cargo run"
