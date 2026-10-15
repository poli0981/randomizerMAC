<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 1024 1024" aria-hidden="true">
  <defs>
    <linearGradient id="logo-bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="#61AFEF" />
      <stop offset="1" stop-color="#4F46E5" />
    </linearGradient>
  </defs>
  <rect x="32" y="32" width="960" height="960" rx="220" fill="url(#logo-bg)" />
  <g fill="none" stroke="#fff" stroke-width="80" stroke-linecap="round" stroke-linejoin="round">
    <path d="M232 712 H322 C392 712 444 686 484 628 L594 468 C634 410 686 360 760 360 H800" />
    <path d="M232 360 H322 C380 360 420 384 456 424" />
    <path d="M564 616 C600 668 650 712 724 712 H800" />
    <path d="M736 292 L804 360 L736 428" />
    <path d="M736 644 L804 712 L736 780" />
  </g>
</svg>
