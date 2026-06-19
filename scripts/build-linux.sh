#!/usr/bin/env bash
# scripts/build-linux.sh
#
# Wrapper around `pnpm tauri build` that works around a known bug in the
# linuxdeploy AppImage bundler:
#
#   linuxdeploy scans /etc/passwd to find every user's home directory and
#   adds <home>/.local/bin to its search path. For the root entry (uid=0)
#   this resolves to /root/.local/bin. If that path is inaccessible the
#   binary throws a C++ exception and aborts the AppImage build.
#
# Fix strategy: compile a tiny LD_PRELOAD shim that intercepts stat/stat64/
# lstat calls for /root/.local/bin and returns success with a fake "empty
# directory" stat structure. No sudo required.
#
# If the C compiler is unavailable, falls back to building only deb+rpm and
# emitting instructions for the one-time manual fix.
#
# Usage:
#   pnpm run tauri:build                            # full build (all targets)
#   pnpm run tauri:build -- --bundles deb,rpm       # deb + rpm only

set -euo pipefail

REAL_HOME="$(eval echo ~"$USER")"
SHIM_DIR="$(mktemp -d /tmp/dev2step-shim.XXXXXX)"
trap 'rm -rf "$SHIM_DIR"' EXIT

SHIM_SRC="$SHIM_DIR/fake_root_local_bin.c"
SHIM_SO="$SHIM_DIR/fake_root_local_bin.so"

cat > "$SHIM_SRC" << 'EOF'
/*
 * LD_PRELOAD shim: make /root/.local/bin appear as an accessible empty
 * directory so linuxdeploy can stat() it without crashing.
 * All other paths are passed through to the real syscall.
 */
#define _GNU_SOURCE
#include <sys/stat.h>
#include <sys/types.h>
#include <dlfcn.h>
#include <string.h>
#include <errno.h>
#include <stdint.h>
#include <time.h>

static const char *TARGET = "/root/.local/bin";

static void fill_fake_dir(struct stat *buf) {
    memset(buf, 0, sizeof(*buf));
    buf->st_mode  = S_IFDIR | 0755;
    buf->st_nlink = 2;
    buf->st_uid   = 0;
    buf->st_gid   = 0;
    buf->st_size  = 4096;
    buf->st_atime = buf->st_mtime = buf->st_ctime = (time_t)0;
}

int stat(const char *path, struct stat *buf) {
    if (strcmp(path, TARGET) == 0) { fill_fake_dir(buf); return 0; }
    static int (*real_stat)(const char *, struct stat *) = NULL;
    if (!real_stat) real_stat = dlsym(RTLD_NEXT, "stat");
    return real_stat(path, buf);
}

int lstat(const char *path, struct stat *buf) {
    if (strcmp(path, TARGET) == 0) { fill_fake_dir(buf); return 0; }
    static int (*real_lstat)(const char *, struct stat *) = NULL;
    if (!real_lstat) real_lstat = dlsym(RTLD_NEXT, "lstat");
    return real_lstat(path, buf);
}

/* Some glibc versions route through __xstat / __lxstat */
int __xstat(int ver, const char *path, struct stat *buf) {
    if (strcmp(path, TARGET) == 0) { fill_fake_dir(buf); return 0; }
    static int (*real)(int, const char *, struct stat *) = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "__xstat");
    return real(ver, path, buf);
}

int __lxstat(int ver, const char *path, struct stat *buf) {
    if (strcmp(path, TARGET) == 0) { fill_fake_dir(buf); return 0; }
    static int (*real)(int, const char *, struct stat *) = NULL;
    if (!real) real = dlsym(RTLD_NEXT, "__lxstat");
    return real(ver, path, buf);
}
EOF

BUILD_OPTS=("$@")

if command -v cc >/dev/null 2>&1; then
    echo "==> Compiling LD_PRELOAD shim to bypass /root/.local/bin permission error..."
    cc -shared -fPIC -o "$SHIM_SO" "$SHIM_SRC" -ldl -Wall -O2
    echo "==> Starting Dev2Step Linux build (all targets)"
    echo "    HOME : $REAL_HOME"
    echo ""
    export HOME="$REAL_HOME"
    export LD_PRELOAD="$SHIM_SO"
    exec pnpm exec tauri build "${BUILD_OPTS[@]}"
else
    echo "WARNING: C compiler not found — cannot compile LD_PRELOAD shim."
    echo ""
    echo "Falling back to deb + rpm only (AppImage skipped)."
    echo ""
    echo "To enable AppImage builds, run once as sudo:"
    echo "  sudo mkdir -p /root/.local/bin"
    echo "  sudo chmod o+rx /root /root/.local /root/.local/bin"
    echo "Then use: pnpm exec tauri build"
    echo ""
    exec pnpm exec tauri build --bundles deb,rpm "${BUILD_OPTS[@]}"
fi
