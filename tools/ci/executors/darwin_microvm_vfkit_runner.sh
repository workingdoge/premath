#!/usr/bin/env sh
set -eu

TASK="${1:-hk-check}"
ROOT="$(CDPATH= cd -- "$(dirname "$0")/../../.." && pwd)"

if [ "$(uname -s)" != "Darwin" ]; then
  echo "darwin_microvm_vfkit_runner is only supported on Darwin hosts." >&2
  exit 2
fi

if ! command -v nix >/dev/null 2>&1; then
  echo "Missing 'nix'. Install Nix with flakes support." >&2
  exit 2
fi

HOST_ARCH="$(uname -m)"
case "$HOST_ARCH" in
  arm64|aarch64)
    GUEST_SYSTEM="aarch64-linux"
    ;;
  x86_64)
    GUEST_SYSTEM="x86_64-linux"
    ;;
  *)
    echo "Unsupported Darwin architecture: $HOST_ARCH" >&2
    exit 2
    ;;
esac

CPUS="${PREMATH_MICROVM_CPUS:-4}"
MEMORY_MB="${PREMATH_MICROVM_MEMORY_MB:-4096}"
HYPERVISOR="${PREMATH_MICROVM_HYPERVISOR:-vfkit}"

RUN_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/premath-microvm-XXXXXX")"
OUT_DIR="$RUN_ROOT/out"
CONTROL_DIR="$RUN_ROOT/control"
mkdir -p "$OUT_DIR" "$CONTROL_DIR"
trap 'rm -rf "$RUN_ROOT"' EXIT INT TERM

printf '%s\n' "$TASK" > "$CONTROL_DIR/task"

cat > "$RUN_ROOT/flake.nix" <<EOF
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    microvm.url = "github:microvm-nix/microvm.nix";
    microvm.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, microvm, ... }: {
    nixosConfigurations.premath-ci = nixpkgs.lib.nixosSystem {
      system = "${GUEST_SYSTEM}";
      modules = [
        microvm.nixosModules.microvm
        ({ ... }: {
          networking.hostName = "premath-ci";

          microvm.hypervisor = "${HYPERVISOR}";
          microvm.vcpu = ${CPUS};
          microvm.mem = ${MEMORY_MB};
          microvm.shares = [
            {
              source = "/nix/store";
              mountPoint = "/nix/.ro-store";
              tag = "ro-store";
              proto = "virtiofs";
            }
            {
              source = "${ROOT}";
              mountPoint = "/workspace";
              tag = "workspace";
              proto = "virtiofs";
            }
            {
              source = "${OUT_DIR}";
              mountPoint = "/host-out";
              tag = "host-out";
              proto = "virtiofs";
            }
            {
              source = "${CONTROL_DIR}";
              mountPoint = "/host-control";
              tag = "host-control";
              proto = "virtiofs";
            }
          ];

          systemd.services.premath-task = {
            description = "Run Premath gate task and shutdown";
            wantedBy = [ "multi-user.target" ];
            after = [ "local-fs.target" ];
            serviceConfig.Type = "oneshot";
            script = ''
              set -eu
              task="\$(cat /host-control/task)"
              cd /workspace
              rc=0
              nix --extra-experimental-features "nix-command flakes" develop /workspace -c sh -lc "cd /workspace && mise run \"\$task\"" || rc=\$?
              printf "%s\n" "\$rc" > /host-out/exit-code
              sync
              poweroff -f
              exit "\$rc"
            '';
          };
        })
      ];
    };
  };
}
EOF

if [ "${PREMATH_MICROVM_DRY_RUN:-0}" = "1" ]; then
  cat "$RUN_ROOT/flake.nix"
  exit 0
fi

nix \
  --extra-experimental-features "nix-command flakes" \
  run "$RUN_ROOT#nixosConfigurations.premath-ci.config.microvm.declaredRunner"

if [ ! -f "$OUT_DIR/exit-code" ]; then
  echo "microvm runner finished without writing /host-out/exit-code" >&2
  exit 2
fi

RC="$(cat "$OUT_DIR/exit-code")"
case "$RC" in
  ''|*[!0-9]*)
    echo "Invalid exit code emitted by microvm task: '$RC'" >&2
    exit 2
    ;;
esac

if [ "$RC" -ne 0 ]; then
  exit "$RC"
fi
