<template>
  <div class="account-stage relative h-full min-h-0">
    <div class="account-art" aria-hidden="true">
      <svg class="account-scene" viewBox="0 0 480 118">
        <path class="scene-ribbon" d="M8 28c70 16 124-26 206-6 74 18 132-12 198 4" />
        <path class="scene-ribbon-soft" d="M36 8c58 16 110-18 184 0 58 14 108-8 160 6" />
        <rect class="scene-square" x="132" y="22" width="15" height="15" rx="3.5" transform="rotate(-16 139 29)" />
        <rect class="scene-square-deep" x="156" y="8" width="11" height="11" rx="2.5" transform="rotate(14 161 13)" />
        <rect class="scene-square" x="116" y="44" width="8" height="8" rx="2" transform="rotate(8 120 48)" />
        <g transform="translate(186 6) rotate(11)">
          <path
            class="scene-body"
            d="M0 30h150a12 12 0 0 1 12 12v50a12 12 0 0 1-12 12H12a12 12 0 0 1-12-12V42a12 12 0 0 1 12-12Z"
          />
          <path class="scene-flap" d="M14 42 81 70l67-28-67 16Z" />
          <g transform="translate(30 -2) rotate(-13)">
            <rect class="scene-letter-shadow" x="1.5" y="2.5" width="96" height="62" rx="8" />
            <rect class="scene-letter" width="96" height="62" rx="8" />
            <path class="scene-lines" d="M14 18h54M14 30h40M14 42h26" />
          </g>
          <path class="scene-pocket" d="M0 62 81 94l81-32v22a12 12 0 0 1-12 12H12A12 12 0 0 1 0 84Z" />
        </g>
      </svg>
    </div>
    <div class="relative z-10 h-full overflow-y-auto px-4 py-6 min-[960px]:px-8">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.account-stage {
  --ribbon: var(--km-art-ribbon);
  --ribbon-soft: var(--km-art-ribbon-soft);
  --square: var(--km-art-square);
  --square-deep: var(--km-art-square-deep);
  --letter: var(--km-art-letter);
  --letter-edge: var(--km-art-letter-edge);
  --letter-line: var(--km-art-letter-line);
  --letter-shadow: var(--km-art-letter-shadow);
  --flap: var(--km-art-flap);
  --body: var(--km-art-body);
  --body-edge: var(--km-art-body-edge);
  --pocket: var(--km-art-pocket);
  background: var(--km-account-wash);
}

.account-art {
  position: absolute;
  inset: 0;
  overflow: hidden;
  pointer-events: none;
}

.account-scene {
  position: absolute;
  top: 0;
  right: 210px;
  width: 460px;
  height: auto;
}

.scene-ribbon,
.scene-ribbon-soft {
  fill: none;
  stroke-linecap: round;
}

.scene-ribbon {
  stroke: var(--ribbon);
  stroke-width: 22;
}

.scene-ribbon-soft {
  stroke: var(--ribbon-soft);
  stroke-width: 16;
}

.scene-square {
  fill: var(--square);
}

.scene-square-deep {
  fill: var(--square-deep);
}

.scene-letter,
.scene-letter-shadow {
  fill: var(--letter);
}

.scene-letter {
  stroke: var(--letter-edge);
  stroke-width: 1;
}

.scene-letter-shadow {
  fill: var(--letter-shadow);
}

.scene-lines {
  fill: none;
  stroke: var(--letter-line);
  stroke-width: 3.5;
  stroke-linecap: round;
}

.scene-flap {
  fill: var(--flap);
}

.scene-body {
  fill: var(--body);
  stroke: var(--body-edge);
  stroke-width: 1.5;
}

.scene-pocket {
  fill: var(--pocket);
}

@media (max-width: 900px) {
  .account-scene {
    right: -36px;
    width: 280px;
    opacity: 0.7;
  }
}
</style>
