// probe.cu — Plan 073 T0a: Ada (4090, sm_89) LDG offered-load-knee probe
//
// Question (pre-registered in riir-clippy .plans/073_offered_load_knee_ldg_probe.md):
//   does the Ada bandwidth-vs-outstanding-plain-load-count knee sit near K~2,
//   as measured on H100 by arXiv:2608.15764 (peak at K~2, -35% by K=8)?
//   Verdict mapping -> T1 landing shape is in the plan; do not re-derive here.
//
// Methodology (paper's, as distilled in Research 121):
//   - fixed total work: every arm issues TOTAL_PER_THREAD 16B plain loads per thread
//   - order-controlled: deterministic stride walk, same order every rep
//   - allocation-sweep: buffer half + base offset rotate per rep (defeats L2 residency)
//   - interleaved reps: arms run round-robin per rep (decorrelates clock/thermal drift;
//     the Bench-728/831 single-A/B-pairs-are-noise discipline)
//   - 2-die control: N/A — Ada is monolithic; recorded here, not silently dropped
//   - ncu: optional; wall-clock bandwidth is the primary instrument
//
// Axis isolation (the T0b dedup finding: this is the paper's unique axis vs B92/B96):
//   TWO fixed grid regimes, per-thread load DEPTH is the only thing that varies:
//     sparse     — 64 blocks x 256 threads = 16384 threads (~128/SM).
//                  Sized so K=1 is BELOW DRAM saturation (Little's law: ~3.2 KB
//                  outstanding/SM needed; K=1 gives ~1.6 KB/SM) so a knee CAN appear.
//     saturating — 512 blocks x 256 threads = 131072 threads (1024/SM resident).
//                  Grid concurrency alone saturates; depth should be flat/negative.
//   If sparse shows a knee and saturating does not, the two axes are separated
//   empirically — that separation IS the rule content.
//
// Consume discipline (load-width + chain-depth):
//   K INDEPENDENT accumulator chains (one per lane), each consuming a 2-deep
//   component tree (x+y)+(z+w) so ALL four f32 lanes stay live — the compiler
//   cannot narrow the float4 load to 8B — while the per-round serial consume
//   is 2 fadds (8 cycles), not K*3 (192 at K=16). Chains tree-reduce once
//   AFTER the loop. Without this, the serial chain would expose dead issue
//   windows between rounds at the sparse grid and deflate arms non-uniformly.
//
// Compile (on the 4090 box):
//   nvcc -O3 -arch=sm_89 -Xptxas -v probe.cu -o probe_873.exe
//   (-Xptxas -v prints registers/thread per K arm; spill at high K would invalidate
//    the arm — record the reg counts in the results note)
// Run:
//   probe_873.exe > ldg_knee_results.csv
//
// Results interpretation (pre-registered):
//   sparse knee at K=2 (+ saturating flat)  -> plan T1 shape (a): land the rule
//   sparse knee elsewhere / absent          -> shape (b): H100-scoped annotation-only
//   curve shape refuted (rising through K=16) -> shape (c): demote-loser record

#include <cstdio>
#include <cstdlib>
#include <cuda_runtime.h>

#define TOTAL_PER_THREAD 4096          // 16B loads per thread, fixed across ALL arms
#define REPS 3                          // interleaved reps per arm
#define NBUF 2                          // allocation-sweep buffer halves (rotate per rep)
#define BUF_BYTES (512ULL << 20)        // 512 MiB per half = 2^25 float4 (power of two)
#define PEAK_GBS 1008.0                 // 4090 measured peak (Bench-684-era figure)

// ---------------------------------------------------------------- kernels ----
// Address map: elem(j) = base_off + tid + j * NTHREADS, j = r*K + k.
//   -> consecutive lanes hit consecutive 16B chunks (fully coalesced 512B/warp);
//   -> the K loads within a round are NTHREADS apart (independent, no aliasing);
//   -> "rounds" group WHEN the consume happens, not the addresses.
// All K loads issue (unrolled) BEFORE any consume; the only dependent chains
// are the K short acc chains, never load->load.

template <int K>
__global__ void probe_kernel(const float4* __restrict__ buf, float* __restrict__ out,
                             int n_rounds, long long buf_mask, long long base_off)
{
    const long long tid = (long long)blockIdx.x * blockDim.x + threadIdx.x;
    const long long N   = (long long)gridDim.x * blockDim.x;
    float accv[K];
    #pragma unroll
    for (int k = 0; k < K; ++k) accv[k] = 0.f;
    long long a = (base_off + tid) & buf_mask;

    for (int r = 0; r < n_rounds; ++r) {
        float4 v[K];
        #pragma unroll
        for (int k = 0; k < K; ++k) {
            v[k] = buf[a];                      // plain cached 16B load
            a = (a + N) & buf_mask;
        }
        #pragma unroll
        for (int k = 0; k < K; ++k)             // K independent 2-fadd chains
            accv[k] += (v[k].x + v[k].y) + (v[k].z + v[k].w);
    }

    float acc = accv[0];
    #pragma unroll
    for (int k = 1; k < K; ++k) acc += accv[k];
    if (acc == -1.f) out[tid] = acc;            // never true; defeats DCE
}

// ---------------------------------------------------------------- host -------
struct Arm { int K; int blocks; int threads; const char* grid_name; };

template <int K>
static void launch_arm(const float4* d_buf, float* d_out, int blocks, int threads,
                       int rep, long long buf_mask, float* ms_out)
{
    const int n_rounds = TOTAL_PER_THREAD / K;   // fixed total work per thread
    // allocation sweep: rotate buffer half + base offset per rep.
    const float4* buf = d_buf + (size_t)(rep % NBUF) * (BUF_BYTES / 16);
    const long long base_off = (long long)(rep * 65537 + 1) * 4096LL;

    cudaEvent_t e0, e1;
    cudaEventCreate(&e0); cudaEventCreate(&e1);
    cudaEventRecord(e0);
    probe_kernel<K><<<blocks, threads>>>(buf, d_out, n_rounds, buf_mask, base_off);
    cudaEventRecord(e1);
    cudaEventSynchronize(e1);
    cudaError_t te = cudaSuccess;
    if (ms_out) te = cudaEventElapsedTime(ms_out, e0, e1);  // warmup passes nullptr: timing deliberately not read
    cudaEventDestroy(e0); cudaEventDestroy(e1);
    if (te != cudaSuccess || (ms_out && *ms_out < 0.f)) {
        printf("# ERROR: timing failed (%s)\n", cudaGetErrorString(te));
        exit(1);                                 // an instrument that fails loudly
    }
}

int main()
{
    int dev = 0;
    cudaDeviceProp prop;
    cudaGetDeviceProperties(&prop, dev);
    cudaSetDevice(dev);
    int clock_khz = 0, mem_clock_khz = 0;   // CUDA 13 removed prop.clockRate/memoryClockRate
    cudaDeviceGetAttribute(&clock_khz, cudaDevAttrClockRate, dev);
    cudaDeviceGetAttribute(&mem_clock_khz, cudaDevAttrMemoryClockRate, dev);
    printf("# device=%s sms=%d l2=%dMB clock_MHz=%d(mem %d)\n",
           prop.name, prop.multiProcessorCount, (int)(prop.l2CacheSize >> 20),
           clock_khz / 1000, mem_clock_khz / 1000);

    const long long buf_mask = (BUF_BYTES / 16) - 1;   // power-of-two mask

    // warmup + clock-ramp: two untimed saturating passes over every K
    {
        float4* d_buf = nullptr; float* d_out = nullptr;
        if (cudaMalloc(&d_buf, NBUF * BUF_BYTES) != cudaSuccess ||
            cudaMalloc(&d_out, 262144 * sizeof(float)) != cudaSuccess) {
            printf("# ERROR: alloc failed\n"); return 1;
        }
        cudaMemset(d_buf, 0x55, NBUF * BUF_BYTES);
        for (int rep = 0; rep < 2; ++rep) {
            launch_arm<1>(d_buf, d_out, 512, 256, rep, buf_mask, nullptr);
            launch_arm<2>(d_buf, d_out, 512, 256, rep, buf_mask, nullptr);
            launch_arm<4>(d_buf, d_out, 512, 256, rep, buf_mask, nullptr);
            launch_arm<8>(d_buf, d_out, 512, 256, rep, buf_mask, nullptr);
            launch_arm<16>(d_buf, d_out, 512, 256, rep, buf_mask, nullptr);
        }
        cudaFree(d_buf); cudaFree(d_out);
    }

    float4* d_buf = nullptr;
    float*  d_out = nullptr;
    if (cudaMalloc(&d_buf, NBUF * BUF_BYTES) != cudaSuccess ||
        cudaMalloc(&d_out, 262144 * sizeof(float)) != cudaSuccess) {
        printf("# ERROR: alloc failed\n"); return 1;
    }
    cudaMemset(d_buf, 0x55, NBUF * BUF_BYTES);

    Arm arms[] = {
        { 1,  64, 256, "sparse"     },
        { 2,  64, 256, "sparse"     },
        { 4,  64, 256, "sparse"     },
        { 8,  64, 256, "sparse"     },
        {16,  64, 256, "sparse"     },
        { 1, 512, 256, "saturating" },
        { 2, 512, 256, "saturating" },
        { 4, 512, 256, "saturating" },
        { 8, 512, 256, "saturating" },
        {16, 512, 256, "saturating" },
    };
    const int NARM = sizeof(arms) / sizeof(arms[0]);

    printf("grid,K,rep,ms,GB_s,pct_peak\n");
    for (int rep = 0; rep < REPS; ++rep) {
        for (int i = 0; i < NARM; ++i) {         // round-robin = interleaved reps
            const Arm& a = arms[i];
            float ms = -1.f;
            switch (a.K) {
                case  1: launch_arm< 1>(d_buf, d_out, a.blocks, a.threads, rep, buf_mask, &ms); break;
                case  2: launch_arm< 2>(d_buf, d_out, a.blocks, a.threads, rep, buf_mask, &ms); break;
                case  4: launch_arm< 4>(d_buf, d_out, a.blocks, a.threads, rep, buf_mask, &ms); break;
                case  8: launch_arm< 8>(d_buf, d_out, a.blocks, a.threads, rep, buf_mask, &ms); break;
                case 16: launch_arm<16>(d_buf, d_out, a.blocks, a.threads, rep, buf_mask, &ms); break;
            }
            const double bytes = (double)a.blocks * a.threads * TOTAL_PER_THREAD * 16.0;
            const double gbs = bytes / ((double)ms * 1e6);
            printf("%s,%d,%d,%.3f,%.1f,%.1f\n", a.grid_name, a.K, rep, ms, gbs,
                   100.0 * gbs / PEAK_GBS);
        }
    }

    cudaFree(d_buf); cudaFree(d_out);

    cudaError_t err = cudaGetLastError();
    if (err != cudaSuccess) { printf("# ERROR: %s\n", cudaGetErrorString(err)); return 1; }
    printf("# done\n");
    return 0;
}
