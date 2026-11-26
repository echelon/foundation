export const legacyModel = 'fal-ai/flux/dev';

const sizes = new Map([
  ['1:1', [1024, 1024]],
  ['4:3', [1024, 768]],
  ['3:4', [768, 1024]],
  ['16:9', [1024, 576]],
  ['9:16', [576, 1024]],
]);
const mediaLimit = 8 * 1024 * 1024;

export const models = new Map([
  [legacyModel, {
    kind: 'image',
    statusPath: 'fal-ai/flux',
    mediaType: 'image/png',
    maxBytes: 2 * 1024 * 1024,
    input({ prompt, count, aspect_ratio: aspect }) {
      const size = sizes.get(aspect);
      if (!size || prompt.length > 4000 || !Number.isInteger(count) || count < 1 || count > 4) return;
      const [width, height] = size;
      return { prompt, image_size: { width, height }, num_images: count, output_format: 'png',
        enable_safety_checker: true };
    },
    files(result) {
      const images = result?.images;
      return Array.isArray(images) && images.length <= 4 ? images.map((item) => item?.url) : [];
    },
  }],
  ['fal-ai/pixverse/v6/text-to-video', {
    kind: 'video',
    statusPath: 'fal-ai/pixverse',
    mediaType: 'video/mp4',
    maxBytes: mediaLimit,
    input({ prompt, duration_seconds: duration, aspect_ratio: aspect, resolution }) {
      if (duration !== 5 || aspect !== '16:9' || resolution !== '720p' || Buffer.byteLength(prompt) > 2048) return;
      return { prompt, duration, aspect_ratio: aspect, resolution, enable_safety_checker: true };
    },
    files: (result) => [result?.video?.url],
  }],
  ['fal-ai/stable-audio-25/text-to-audio', {
    kind: 'audio',
    statusPath: 'fal-ai/stable-audio-25',
    mediaType: 'audio/wav',
    mediaAliases: ['audio/x-wav', 'audio/wave'],
    maxBytes: mediaLimit,
    input({ prompt, duration_seconds: duration, format }) {
      if (![5, 10].includes(duration) || format !== 'wav' || Buffer.byteLength(prompt) > 2048) return;
      return { prompt, seconds_total: duration, enable_safety_checker: true };
    },
    // fal documents a File object and also shows a bare URL in its example.
    files: (result) => [typeof result?.audio === 'string' ? result.audio : result?.audio?.url],
  }],
  ['fal-ai/meshy/v6/text-to-3d', {
    kind: 'mesh',
    statusPath: 'fal-ai/meshy',
    mediaType: 'model/gltf-binary',
    maxBytes: mediaLimit,
    input({ prompt, format }) {
      if (format !== 'glb' || [...prompt].length > 600) return;
      return { prompt, mode: 'preview', should_remesh: true, target_polycount: 10_000, enable_safety_checker: true };
    },
    files: (result) => [result?.model_glb?.url],
  }],
]);
