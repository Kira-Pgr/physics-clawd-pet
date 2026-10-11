import { DEFAULT_TUNING } from '../engine/params';
import { Pet } from '../engine/pet';
import { ParticleSystem } from '../engine/particles';
import { Renderer } from '../render/renderer';
import { SKINS } from '../skin/registry';
import type { Skin } from '../skin/types';

function canvasFor(skin: Skin, scale: number, width: number, height: number, mode?: 'petted' | 'laptop') {
  const canvas = document.createElement('canvas');
  const tuning = { ...DEFAULT_TUNING, petScale: scale, activityChance: 0, trail: 0 };
  const pet = new Pet(tuning, skin.grid, { left: 0, top: 0, right: width, bottom: height - 16 }, () => 0.5);
  pet.setGrid(skin.grid, { pixelArt: skin.pixelArt, actions: skin.actions });
  pet.placeOnFloor(width / 2);
  pet.t = 1.2;
  if (mode) { pet.perform(mode); pet.modeTime = mode === 'laptop' ? 0.8 : 0.3; }
  const renderer = new Renderer(canvas, skin);
  renderer.resize(width, height, 2);
  renderer.draw(pet, new ParticleSystem(() => 0.5), tuning, 0);
  return canvas;
}

const order = ['dots', 'dots-green', 'dots-yellow', 'dots-pink'];
for (const skin of SKINS.filter((s) => order.includes(s.id)).sort((a, b) => order.indexOf(a.id) - order.indexOf(b.id))) {
  const card = document.createElement('article');
  const title = document.createElement('h2');
  title.textContent = skin.name;
  card.append(title);
  const stage = document.createElement('div');
  stage.className = 'stage';
  stage.append(canvasFor(skin, 14, 330, 290));
  card.append(stage);
  const label = document.createElement('p');
  label.className = 'label';
  label.textContent = 'DESKTOP SIZE · IDLE / PETTED';
  card.append(label);
  const small = document.createElement('div');
  small.className = 'small';
  const row = document.createElement('div');
  row.style.display = 'flex';
  for (const mode of [undefined, 'petted'] as const) {
    const canvas = canvasFor(skin, 6, 165, 130, mode);
    canvas.style.width = '50%';
    row.append(canvas);
  }
  small.append(row);
  const working = document.createElement('p');
  working.className = 'label';
  working.textContent = 'WORKING';
  small.append(working, canvasFor(skin, 6, 330, 130, 'laptop'));
  card.append(small);
  document.getElementById('gallery')!.append(card);
}
document.documentElement.dataset.ready = 'true';
