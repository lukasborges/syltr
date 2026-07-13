#!/usr/bin/env bash
#
# Compila o CEF COM codecs proprietários (H.264/AAC) — necessário para tocar
# vídeo do WhatsApp Web, que é sempre H.264/AAC em MP4.
#
# Por quê: o CEF que o crate `cef` baixa do CDN público (Spotify) é compilado com
# `proprietary_codecs=false`. Nesse build o Chromium nem *reconhece* H.264
# (`canPlayType('video/mp4; codecs="avc1..."')` → ""), então o WhatsApp mostra
# "não é possível reproduzir o vídeo" antes de decodificar. Isso é compile-time
# (flag GN no //media, dentro do libcef.so) — nenhum switch de runtime resolve.
#
# Este é um build ONE-TIME: compila o Chromium inteiro (horas, ~100 GB de disco).
# Depois é só reusar a dist gerada apontando CEF_PATH pra ela:
#
#     ./scripts/build-cef-codecs.sh
#     export CEF_PATH=<dist impressa no fim>
#     ./scripts/install-app.sh          # (ou: cargo build --release)
#
# LICENCIAMENTO: H.264/AAC são patenteados. Por isso o pacote publicado/AUR
# continua codec-free — habilitar codecs é um ato LOCAL do usuário (este script).
# Não redistribua o libcef.so resultante.
#
# Rerode este script só ao trocar a versão do CEF (ver bloco "VERSÃO-ALVO").
set -euo pipefail

# ---- VERSÃO-ALVO (tem que casar EXATAMENTE com o crate `cef` do Cargo.lock) ----
# A ABI dos bindings do cef-dll-sys é gerada para esta versão; um libcef.so de
# outra versão crasha. Ao subir a major do CEF no Cargo.toml, atualize aqui:
#   CEF_COMMIT  = hash curto do commit CEF (do sufixo +g<hash> da versão)
#   CHROMIUM_BR = nº do branch do Chromium (149.0.7827.201 → 7827)
CEF_VERSION="149.0.6"
CEF_COMMIT="0d0eeb6"
CHROMIUM_VERSION="149.0.7827.201"
CHROMIUM_BR="7827"

BUILD_DIR="${CEF_BUILD_DIR:-$HOME/.cache/syltr-cef-build}"
AUTOMATE_URL="https://raw.githubusercontent.com/chromiumembedded/cef/master/tools/automate/automate-git.py"

log() { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
err() { printf '\033[1;31mErro:\033[0m %s\n' "$*" >&2; exit 1; }

# ---- Sanidade: a versão daqui bate com a que o Syltr espera? ----
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
if [ -f "$ROOT/Cargo.lock" ]; then
    locked=$(grep -A1 '^name = "cef-dll-sys"' "$ROOT/Cargo.lock" | grep version \
             | grep -oE '\+[0-9]+\.[0-9]+\.[0-9]+' | tr -d '+' | head -1 || true)
    if [ -n "$locked" ] && [ "$locked" != "$CEF_VERSION" ]; then
        err "Cargo.lock espera CEF $locked, mas este script mira $CEF_VERSION.
     Atualize o bloco VERSÃO-ALVO em $(basename "$0") (commit/branch/versão)."
    fi
fi

# ---- Pré-requisitos ----
log "Checando pré-requisitos…"
for t in git python3 curl; do
    command -v "$t" >/dev/null 2>&1 || err "'$t' não encontrado (instale-o)."
done
avail_kb=$(df -Pk "$(dirname "$BUILD_DIR")" 2>/dev/null | awk 'NR==2{print $4}' || echo 0)
if [ "${avail_kb:-0}" -lt $((90*1024*1024)) ]; then
    printf '\033[1;33mAviso:\033[0m menos de ~90 GB livres em %s. O build do Chromium precisa de ~100 GB.\n' \
        "$(dirname "$BUILD_DIR")"
    printf 'Continuar mesmo assim? [s/N] '
    read -r ans; [ "$ans" = "s" ] || [ "$ans" = "S" ] || exit 1
fi

cat <<EOF

  Vai compilar o CEF $CEF_VERSION (Chromium $CHROMIUM_VERSION) COM H.264/AAC.
  Diretório de trabalho : $BUILD_DIR
  Isto leva HORAS e baixa dezenas de GB. Ctrl-C para abortar.

EOF

mkdir -p "$BUILD_DIR"
cd "$BUILD_DIR"

# ---- automate-git.py (o CEF orquestra o fetch+patch+build do Chromium) ----
if [ ! -f automate-git.py ]; then
    log "Baixando automate-git.py…"
    curl -fsSL "$AUTOMATE_URL" -o automate-git.py
fi

# ---- GN defines: AQUI é onde os codecs entram ----
export CEF_USE_GN=1
export GN_DEFINES="proprietary_codecs=true ffmpeg_branding=Chrome is_official_build=true"
# Se o build oficial falhar por PGO, remova is_official_build=true acima (gera um
# release não-oficial, funcionalmente equivalente para os codecs).

log "Rodando automate-git.py (CEF $CEF_VERSION, codecs on)…"
# NOTA: as flags do automate-git variam por versão. Confira antes com:
#   python3 automate-git.py --help
# --minimal-distrib-only: gera só a dist minimal (o que o cef-dll-sys consome);
#   evita client/sandbox/tools e o target cefsimple.
# --with-pgo-profiles: necessário porque is_official_build=true usa PGO.
python3 automate-git.py \
    --download-dir="$BUILD_DIR" \
    --branch="$CHROMIUM_BR" \
    --checkout="$CEF_COMMIT" \
    --x64-build \
    --with-pgo-profiles \
    --no-debug-build \
    --minimal-distrib-only \
    --force-clean

# ---- Localizar a dist minimal gerada ----
log "Procurando a distribuição minimal gerada…"
DIST=$(find "$BUILD_DIR" -type d -name 'cef_binary_*linux64_minimal' 2>/dev/null \
       | sort | tail -1 || true)
[ -n "$DIST" ] && [ -f "$DIST/libcef.so" ] || \
    err "dist minimal não encontrada (procure por *_linux64_minimal com libcef.so em $BUILD_DIR)."

# ---- Gerar archive.json (o cef-dll-sys build.rs valida a versão por aqui) ----
# check_archive_json lê só o campo "name" (versão ≤ esperada passa); sha1 não é
# verificado ao usar um dir local via CEF_PATH.
DIST_NAME="$(basename "$DIST").tar.bz2"
cat > "$DIST/archive.json" <<JSON
{
  "type": "minimal",
  "name": "$DIST_NAME",
  "sha1": "0000000000000000000000000000000000000000"
}
JSON

# ---- Confirmar que os codecs estão de fato no binário ----
if command -v strings >/dev/null 2>&1; then
    if strings -a "$DIST/libcef.so" | grep -q "H.264 / AVC"; then
        log "libcef.so contém o decoder H.264 ✓"
    fi
fi

printf '\n\033[1;32m✓ CEF com codecs compilado.\033[0m\n'
cat <<EOF

  Dist: $DIST

  Use assim (dev):
      export CEF_PATH="$DIST"
      ./scripts/install-app.sh            # ou: cargo build --release
      # rodar direto do target:
      CEF_PATH=target/release LD_LIBRARY_PATH=target/release target/release/syltr

  Empacotar com codec (opt-in, NÃO publicar):
      CEF_PATH="$DIST" makepkg -f        # dentro de packaging/

  Verificar codec (com SYLTR_DEBUG=1 rodando, na página do WhatsApp):
      node scripts/cef-codec-check.js
EOF
