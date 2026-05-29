#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE_DIR="${SCRIPT_DIR}/../crates/tune_engine/benches/fixtures"

CORE_FIXTURES=(arithmetic.tn tuple_expr.tn struct_methods.tn)
FLOW_FIXTURES=(finite_for.tn structural_match.tn spawn_join.tn)
GENERIC_FIXTURES=(generic_identity.tn)
ALL_FIXTURES=("${CORE_FIXTURES[@]}" "${FLOW_FIXTURES[@]}" "${GENERIC_FIXTURES[@]}")

usage() {
  cat <<'USAGE'
Usage: benchmark_pipeline.sh <bench|flamegraph|quality|trace|compare|ci> [args...]
       benchmark_pipeline.sh trace [--full] [--csv] [--strict-shapes]
                                      [--family <name>] [--emit-baseline <file>] [--compare <file>]
                                      [--max-stage-delta-pct <pct>] [--max-counter-delta-pct <pct>]
                                      <source.tn...>

bench
  Run tune_engine pipeline benchmarks (frontend and full).
  Optional: pass a benchmark filter as one extra arg to target one case.
  Example:
    benchmark_pipeline.sh bench tune_pipeline_frontend_profile/frontend/arithmetic

flamegraph
  Record a flamegraph of the pipeline benchmark binary:
    benchmark_pipeline.sh flamegraph tune_pipeline_full_profile/full/structural_match

quality <source.tn...>
  Run IR-quality checks on one or more Tune sources:
    benchmark_pipeline.sh quality app/main.tn path/to/file.tn

trace
  Print stage timings and IR-quality for source files:
    benchmark_pipeline.sh trace crates/tune_engine/benches/fixtures/arithmetic.tn
    benchmark_pipeline.sh trace --full --csv path/to/source.tn

compare
  Compare current trace against baseline CSV:
    benchmark_pipeline.sh compare --compare scripts/bench_baseline.csv crates/tune_engine/benches/fixtures/arithmetic.tn
    benchmark_pipeline.sh compare --compare scripts/bench_baseline.csv --family core

ci
  Run performance checks for default fixture families:
    benchmark_pipeline.sh ci [--compare scripts/bench_baseline.csv] [--family all]

fixture families: core, flow, generic, all (defaults to all)
USAGE
}

resolve_family() {
  local family="$1"

  case "$family" in
    core)
      printf '%s\n' "${CORE_FIXTURES[@]}"
      ;;
    flow)
      printf '%s\n' "${FLOW_FIXTURES[@]}"
      ;;
    generic)
      printf '%s\n' "${GENERIC_FIXTURES[@]}"
      ;;
    all)
      printf '%s\n' "${ALL_FIXTURES[@]}"
      ;;
    *)
      return 1
      ;;
  esac
}

build_paths() {
  local family="$1"
  shift
  local -a effective

  if [ "$#" -eq 0 ]; then
    effective=()
    local resolved_lines
    if ! resolved_lines="$(resolve_family "$family")"; then
      return 1
    fi
    if [ -z "${resolved_lines}" ]; then
      return 1
    fi
    while IFS= read -r path; do
      effective+=("$path")
    done <<< "${resolved_lines}"
  else
    effective=("$@")
  fi

  for path in "${effective[@]}"; do
    if [[ "$path" == *.tn ]]; then
      if [[ "$path" == *"/"* ]]; then
        printf '%s\n' "$path"
      else
        printf '%s\n' "${FIXTURE_DIR}/${path}"
      fi
    else
      printf '%s\n' "$path"
    fi
  done
}

command="${1:-}"
if [ -z "$command" ]; then
  usage
  exit 1
fi
shift || true

case "$command" in
  bench)
    echo "running criterion pipeline benches"
    if [ "$#" -eq 0 ]; then
      cargo bench -p tune_engine --bench pipeline
    else
      cargo bench -p tune_engine --bench pipeline -- "$1"
    fi
    ;;
  flamegraph)
    if ! command -v cargo-flamegraph >/dev/null 2>&1; then
      echo "cargo-flamegraph is required (cargo install flamegraph)" >&2
      exit 1
    fi
    echo "recording flamegraph: target/criterion/reports/pipeline_frontend_profile/index.html and friends"
    if [ "$#" -eq 0 ]; then
      cargo flamegraph -p tune_engine --bench pipeline
    else
      cargo flamegraph -p tune_engine --bench pipeline -- "$1"
    fi
    ;;
  trace)
    declare -a args paths resolved
    family="all"
    resolved_output=""
    args=()
    paths=()
    while [ "$#" -gt 0 ]; do
      arg="$1"
      shift
      case "$arg" in
        --family)
          if [ "$#" -lt 1 ]; then
            echo "missing family for --family" >&2
            usage
            exit 1
          fi
          family="$1"
          shift
          ;;
        --full|--csv|--strict-shapes)
          args+=("$arg")
          ;;
        --emit-baseline|--compare|--max-stage-delta-pct|--max-counter-delta-pct)
          if [ "$#" -lt 1 ]; then
            echo "$arg requires a value" >&2
            usage
            exit 1
          fi
          if [[ "$1" == --* ]]; then
            echo "$arg requires a value" >&2
            usage
            exit 1
          fi
          args+=("$arg" "$1")
          shift
          ;;
        --)
          paths+=("$@")
          break
          ;;
        -*)
          echo "unknown argument: $arg" >&2
          usage
          exit 1
          ;;
        *)
          paths+=("$arg")
          ;;
      esac
    done

    if [ "${#paths[@]}" -eq 0 ]; then
      resolved_status=0
      resolved_output="$(build_paths "$family")" || resolved_status=$?
    else
      resolved_status=0
      resolved_output="$(build_paths "$family" "${paths[@]}")" || resolved_status=$?
    fi
    if [ "$resolved_status" -ne 0 ]; then
      echo "unknown family: $family" >&2
      usage
      exit 1
    fi
    resolved=()
    while IFS= read -r path; do
      resolved+=("$path")
    done <<< "${resolved_output}"

    if [ "${#resolved[@]}" -eq 0 ]; then
      echo "no paths resolved for family: $family" >&2
      exit 1
    fi

    if [ "${#args[@]}" -gt 0 ]; then
      cargo run --package tune_engine --bin profile_trace -- "${args[@]}" "${resolved[@]}"
    else
      cargo run --package tune_engine --bin profile_trace -- "${resolved[@]}"
    fi
    ;;
  compare)
    declare -a args paths resolved
    has_compare=0
    family="all"
    args=()
    paths=()
    while [ "$#" -gt 0 ]; do
      arg="$1"
      shift
      case "$arg" in
        --family)
          if [ "$#" -lt 1 ]; then
            echo "missing family for --family" >&2
            usage
            exit 1
          fi
          family="$1"
          shift
          ;;
        --full|--csv|--strict-shapes)
          args+=("$arg")
        ;;
        --emit-baseline|--compare|--max-stage-delta-pct|--max-counter-delta-pct)
          if [ "$#" -lt 1 ]; then
            echo "$arg requires a value" >&2
            usage
            exit 1
          fi
          if [[ "$1" == --* ]]; then
            echo "$arg requires a value" >&2
            usage
            exit 1
          fi
          if [ "$arg" = --compare ]; then
            has_compare=1
          fi
          args+=("$arg" "$1")
          shift
          ;;
        --)
          paths+=("$@")
          break
          ;;
        -*)
          echo "unknown argument: $arg" >&2
          usage
          exit 1
          ;;
        *)
          paths+=("$arg")
          ;;
      esac
    done

    if [ "$has_compare" -eq 0 ]; then
      echo "compare command requires --compare <baseline_csv>" >&2
      usage
      exit 1
    fi

    if [ "${#paths[@]}" -eq 0 ]; then
      resolved_status=0
      resolved_output="$(build_paths "$family")" || resolved_status=$?
    else
      resolved_status=0
      resolved_output="$(build_paths "$family" "${paths[@]}")" || resolved_status=$?
    fi
    if [ "$resolved_status" -ne 0 ]; then
      echo "unknown family: $family" >&2
      usage
      exit 1
    fi
    resolved=()
    while IFS= read -r path; do
      resolved+=("$path")
    done <<< "${resolved_output}"

    if [ "${#resolved[@]}" -eq 0 ]; then
      echo "no paths resolved for family: $family" >&2
      exit 1
    fi

    if [ "${#args[@]}" -gt 0 ]; then
      cargo run --package tune_engine --bin profile_trace -- "${args[@]}" "${resolved[@]}"
    else
      cargo run --package tune_engine --bin profile_trace -- "${resolved[@]}"
    fi
    ;;
  ci)
    declare -a resolved
    baseline=""
    family="all"
    if [ "$#" -gt 0 ] && [ "$1" == --compare ]; then
      if [ "$#" -lt 2 ]; then
        echo "--compare requires a baseline csv path in ci" >&2
        exit 1
      fi
      baseline="$2"
      shift 2
    fi

    if [ "$#" -gt 0 ] && [ "$1" == --family ]; then
      if [ "$#" -lt 2 ]; then
        echo "--family requires a value in ci" >&2
        exit 1
      fi
      family="$2"
      shift 2
    fi

    if [ "$#" -gt 0 ]; then
      echo "ci accepts only --compare and --family" >&2
      exit 1
    fi

    if ! resolved_output="$(build_paths "$family")"; then
      echo "unknown family: $family" >&2
      usage
      exit 1
    fi
    resolved=()
    while IFS= read -r path; do
      resolved+=("$path")
    done <<< "${resolved_output}"
    if [ "${#resolved[@]}" -eq 0 ]; then
      echo "no paths resolved for family: $family" >&2
      exit 1
    fi

    if [ -n "$baseline" ]; then
      cargo run --package tune_engine --bin profile_trace -- \
        --compare "$baseline" \
        --full \
        --max-stage-delta-pct 10 \
        --max-counter-delta-pct 10 \
        "${resolved[@]}"
    else
      cargo run --package tune_engine --bin quality_check -- "${resolved[@]}" --strict-shapes
    fi
    ;;
  quality)
    if [ "$#" -eq 0 ]; then
      echo "quality requires at least one source path" >&2
      usage
      exit 1
    fi
    cargo run --package tune_engine --bin quality_check -- "$@" --strict-shapes
    ;;
  *)
    usage
    exit 1
    ;;
esac
