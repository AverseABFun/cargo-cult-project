#!/bin/sh
# shellcheck shell=dash

:

{ #This block of code taken from https://github.com/rust-lang/rustup/blob/7ccf717e6e1aee46f65cc6fea4132a3f0e37593b/rustup-init.sh

    _ansi_escapes_are_valid=false
    if [ -t 2 ]; then
        if [ "${TERM+set}" = 'set' ]; then
            case "$TERM" in
            xterm* | rxvt* | urxvt* | linux* | vt*)
                _ansi_escapes_are_valid=true
                ;;
            esac
        fi
    fi

    check_cmd() {
        command -v "$1" >/dev/null 2>&1
    }

    err() {
        __print 'error' "$1" >&2
    }

    __print() {
        if $_ansi_escapes_are_valid; then
            printf '\33[1m%s:\33[0m %s\n' "$1" "$2" >&2
        else
            printf '%s: %s\n' "$1" "$2" >&2
        fi
    }

    need_cmd() {
        if ! check_cmd "$1"; then
            err "need '$1' (command not found)"
        fi
    }

    get_current_exe() {
        # Returns the executable used for system architecture detection
        # This is only run on Linux
        if test -L /proc/self/exe; then
            _current_exe=/proc/self/exe
        else
            warn "Unable to find /proc/self/exe. System architecture detection might be inaccurate."
            if test -n "$SHELL"; then
                _current_exe=$SHELL
            else
                need_cmd /bin/sh
                _current_exe=/bin/sh
            fi
            warn "Falling back to $_current_exe."
        fi
        echo "$_current_exe"
    }

    get_bitness() {
        need_cmd head
        # Architecture detection without dependencies beyond coreutils.
        # ELF files start out "\x7fELF", and the following byte is
        #   0x01 for 32-bit and
        #   0x02 for 64-bit.
        # The printf builtin on some shells like dash only supports octal
        # escape sequences, so we use those.
        _current_exe_head=$(head -c 5 "$_current_exe")
        if [ "$_current_exe_head" = "$(printf '\177ELF\001')" ]; then
            echo 32
        elif [ "$_current_exe_head" = "$(printf '\177ELF\002')" ]; then
            echo 64
        else
            err "unknown platform bitness"
            exit 1
        fi
    }

    get_architecture() {
        _ostype="$(uname -s)"
        _cputype="$(uname -m)"
        _clibtype="gnu"

        if [ "$_ostype" = Linux ]; then
            if [ "$(uname -o)" = Android ]; then
                _ostype=Android
            fi
            if ldd --version 2>&1 | grep -q 'musl'; then
                _clibtype="musl"
            fi
        fi

        if [ "$_ostype" = Darwin ]; then
            # Darwin `uname -m` can lie due to Rosetta shenanigans. If you manage to
            # invoke a native shell binary and then a native uname binary, you can
            # get the real answer, but that's hard to ensure, so instead we use
            # `sysctl` (which doesn't lie) to check for the actual architecture.
            if [ "$_cputype" = i386 ]; then
                # Handling i386 compatibility mode in older macOS versions (<10.15)
                # running on x86_64-based Macs.
                # Starting from 10.15, macOS explicitly bans all i386 binaries from running.
                # See: <https://support.apple.com/en-us/HT208436>

                # Avoid `sysctl: unknown oid` stderr output and/or non-zero exit code.
                if sysctl hw.optional.x86_64 2>/dev/null || true | grep -q ': 1'; then
                    _cputype=x86_64
                fi
            elif [ "$_cputype" = x86_64 ]; then
                # Handling x86-64 compatibility mode (a.k.a. Rosetta 2)
                # in newer macOS versions (>=11) running on arm64-based Macs.
                # Rosetta 2 is built exclusively for x86-64 and cannot run i386 binaries.

                # Avoid `sysctl: unknown oid` stderr output and/or non-zero exit code.
                if sysctl hw.optional.arm64 2>/dev/null || true | grep -q ': 1'; then
                    _cputype=arm64
                fi
            fi
        fi

        if [ "$_ostype" = SunOS ]; then
            # Both Solaris and illumos presently announce as "SunOS" in "uname -s"
            # so use "uname -o" to disambiguate.  We use the full path to the
            # system uname in case the user has coreutils uname first in PATH,
            # which has historically sometimes printed the wrong value here.
            if [ "$(/usr/bin/uname -o)" = illumos ]; then
                _ostype=illumos
            fi

            # illumos systems have multi-arch userlands, and "uname -m" reports the
            # machine hardware name; e.g., "i86pc" on both 32- and 64-bit x86
            # systems.  Check for the native (widest) instruction set on the
            # running kernel:
            if [ "$_cputype" = i86pc ]; then
                _cputype="$(isainfo -n)"
            fi
        fi

        case "$_ostype" in

        Android)
            _ostype=linux-android
            ;;

        Linux)
            _current_exe=$(get_current_exe)
            _ostype=unknown-linux-$_clibtype
            _bitness=$(get_bitness "$_current_exe")
            ;;

        FreeBSD)
            _ostype=unknown-freebsd
            ;;

        NetBSD)
            _ostype=unknown-netbsd
            ;;

        DragonFly)
            _ostype=unknown-dragonfly
            ;;

        Darwin)
            _ostype=apple-darwin
            ;;

        illumos)
            _ostype=unknown-illumos
            ;;

        MINGW* | MSYS* | CYGWIN* | Windows_NT)
            _ostype=pc-windows-gnu
            ;;

        *)
            err "unrecognized OS type: $_ostype"
            exit 1
            ;;

        esac

        case "$_cputype" in

        i386 | i486 | i686 | i786 | x86)
            _cputype=i686
            ;;

        xscale | arm)
            _cputype=arm
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            fi
            ;;

        armv6l)
            _cputype=arm
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            else
                _ostype="${_ostype}eabihf"
            fi
            ;;

        armv7l | armv8l)
            _cputype=armv7
            if [ "$_ostype" = "linux-android" ]; then
                _ostype=linux-androideabi
            else
                _ostype="${_ostype}eabihf"
            fi
            ;;

        aarch64 | arm64)
            _cputype=aarch64
            ;;

        x86_64 | x86-64 | x64 | amd64)
            _cputype=x86_64
            ;;

        mips)
            _cputype=$(get_endianness "$_current_exe" mips '' el)
            ;;

        mips64)
            if [ "$_bitness" -eq 64 ]; then
                # only n64 ABI is supported for now
                _ostype="${_ostype}abi64"
                _cputype=$(get_endianness "$_current_exe" mips64 '' el)
            fi
            ;;

        ppc)
            _cputype=powerpc
            ;;

        ppc64)
            _cputype=powerpc64
            ;;

        ppc64le)
            _cputype=powerpc64le
            ;;

        s390x)
            _cputype=s390x
            ;;
        riscv64)
            _cputype=riscv64gc
            ;;
        loongarch64)
            _cputype=loongarch64
            ensure_loongarch_uapi
            ;;
        *)
            err "unknown CPU type: $_cputype"
            exit 1
            ;;

        esac

        # Detect 64-bit linux with 32-bit userland
        if [ "${_ostype}" = unknown-linux-gnu ] && [ "${_bitness}" -eq 32 ]; then
            case $_cputype in
            x86_64)
                if [ -n "${RUSTUP_CPUTYPE:-}" ]; then
                    _cputype="$RUSTUP_CPUTYPE"
                else {
                    # 32-bit executable for amd64 = x32
                    if is_host_amd64_elf "$_current_exe"; then {
                        err "This host is running an x32 userland, for which no native toolchain is provided."
                        err "You will have to install multiarch compatibility with i686 or amd64."
                        err "To do so, set the RUSTUP_CPUTYPE environment variable set to i686 or amd64 and re-run this script."
                        err "You will be able to add an x32 target after installation by running \`rustup target add x86_64-unknown-linux-gnux32\`."
                        exit 1
                    }; else
                        _cputype=i686
                    fi
                }; fi
                ;;
            mips64)
                _cputype=$(get_endianness "$_current_exe" mips '' el)
                ;;
            powerpc64)
                _cputype=powerpc
                ;;
            aarch64)
                _cputype=armv7
                if [ "$_ostype" = "linux-android" ]; then
                    _ostype=linux-androideabi
                else
                    _ostype="${_ostype}eabihf"
                fi
                ;;
            riscv64gc)
                err "riscv64 with 32-bit userland unsupported"
                exit 1
                ;;
            esac
        fi

        # Detect armv7 but without the CPU features Rust needs in that build,
        # and fall back to arm.
        # See https://github.com/rust-lang/rustup.rs/issues/587.
        if [ "$_ostype" = "unknown-linux-gnueabihf" ] && [ "$_cputype" = armv7 ]; then
            if ! (ensure grep '^Features' /proc/cpuinfo | grep -E -q 'neon|simd'); then
                # Either `/proc/cpuinfo` is malformed or unavailable, or
                # at least one processor does not have NEON (which is asimd on armv8+).
                _cputype=arm
            fi
        fi

        _arch="${_cputype}-${_ostype}"

        export RETVAL="$_arch"
    }

}

# shellcheck disable=SC2153
test -n "$FISH_VERSION" && echo "You appear to be running fish, please use install.fish instead of install.sh!" && exit 1
# the above is kind of cursed, but I can't use if statements because fish likes to be different

START_DIR="$(pwd)"
cd "$(dirname "$0")"

set -euo pipefail

# shellcheck shell=bash
if [ -n "$BASH_VERSION" ] || [ -n "$ZSH_VERSION" ]; then
    HAS_TRAPS=0
    # shellcheck disable=SC3045
    if ! type -a "trap" 2>/dev/null | grep -E '\<built-?in\>' >/dev/null; then
        HAS_TRAPS=1
    fi
fi

workdir=$(
    echo 'mkstemp(template)' |
        m4 -D template="${TMPDIR:-/tmp}/rust_pkg_gen_installerXXXXXX"
) || exit

cleanup() {
    cd "$START_DIR"
    if [ -n "$workdir" ]; then
        return
    fi
    rm -rf "$workdir"
}
if [ "$HAS_TRAPS" = "1" ]; then
    trap SIGINT cleanup
    trap SIGKILL cleanup
fi

PARENT_DIR=$(
    cd dirname "$0"
    pwd
)

COMPONENTS="${COMPONENTS:-&?TOOLCHAIN.COMPONENTS}"
CHANNEL="${CHANNEL:-&?TOOLCHAIN.CHANNEL}"
FORMAT="${FORMAT:-.tar.gz}"
TAR_FLAGS="${TAR_FLAGS:-"-xf"}"

USING_PKG="${USING_PKG:-&?TOOLCHAIN.PKG}"

if ! command -v tar >/dev/null 2>&1; then
    echo "Tar is not installed, cannot unpack archives!"
    cleanup
    exit 1
fi

if [ "$FORMAT" = ".tar.gz" ]; then
    if ! tar --help | grep gzip >/dev/null 2>&1; then
        if ! tar --help | grep xz >/dev/null 2>&1; then
            echo "Tar does not support gzip or xz, cannot unpack archives!"
            cleanup
            exit 1
        fi
        echo "Tar supports xz but not gzip, switching format to .tar.xz from .tar.gz!"
        FORMAT=".tar.xz"
    fi
fi

if [ "$FORMAT" = ".tar.xz" ]; then
    tar --help | grep xz >/dev/null 2>&1
    if ! tar --help | grep xz >/dev/null 2>&1; then
        if ! tar --help | grep gzip >/dev/null 2>&1; then
            echo "Tar does not support gzip or xz, cannot unpack archives!"
            cleanup
            exit 1
        fi
        echo "Tar supports gzip but not xz, switching format to .tar.gz from .tar.xz!"
        FORMAT=".tar.gz"
    fi
fi

if [ "$FORMAT" = ".tar.gz" ]; then
    TAR_FLAGS="--gzip $TAR_FLAGS"
fi

if [ "$FORMAT" = ".tar.xz" ]; then
    TAR_FLAGS="--xz $TAR_FLAGS"
fi

target_triple=$(get_architecture)

if [ "$_cputype" = "apple-darwin" ] && [ -n "$USING_PKG" ]; then
    installer -pkg "$PARENT_DIR/toolchain/rust-$CHANNEL-$target_triple.pkg" -target CurrentUserHomeDirectory
fi

cd "$workdir"
for comp in $COMPONENTS; do
    if [ ! -e "$PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT" ]; then
        if [ -n "$IGNORE_NONEXISTANT_COMPONENTS" ]; then
            continue
        fi
        echo "Expected file $PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT to exist, but doesn't!"
        echo "(hint: to ignore and continue, set the environment variable IGNORE_NONEXISTANT_COMPONENTS to 1)"
        cleanup
        exit 1
    fi
    if [ -z "$DONT_HASH" ]; then
        if [ ! -e "$PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT.sha256" ]; then
            echo "Expected file $PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT.sha256 to exist, but doesn't!"
            echo "(hint: to disable this warning, set the environment variable DONT_HASH to 1)"
            cleanup
            exit 1
        fi
        HASH1=$(openssl dgst -sha256 -hex "$PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT" | awk '{print $2}')
        HASH2=$(cat "$PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT.sha256")
        if [ "$HASH1" != "$HASH2" ]; then
            echo "Error: Hash of $PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT does not match expected hash!"
            echo "(hint: to disable this warning, set the environment variable DONT_HASH to 1)"
            cleanup
            exit 1
        fi
        echo "Tested - hash and expected hash of $comp-$CHANNEL-$target_triple.$FORMAT match($HASH1)!"
    fi
    mkdir "$comp-$CHANNEL-$target_triple"
    cd "$comp-$CHANNEL-$target_triple"
    # shellcheck disable=SC2086
    tar $TAR_FLAGS "$PARENT_DIR/toolchain/$comp-$CHANNEL-$target_triple.$FORMAT"
    cd "$comp-$CHANNEL-$target_triple"
    chmod u+x install.sh
    if [ ! -x "install.sh" ]; then
        echo "Cannot execute $(pwd)/install.sh (likely because of mounting /tmp as noexec)."
        echo "Set \$TMPDIR to a path that you can execute files in and rerun this script."
        echo "Note that this will create a temporary directory inside this path and delete it at the end of the script."
        cleanup
        exit 1
    fi
    ./install.sh
done

cleanup
