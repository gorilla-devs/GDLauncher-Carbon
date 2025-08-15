import { createFileRoute } from '@tanstack/solid-router';
import { Progressbar } from '../../../../src';
import { createSignal, createEffect, onCleanup } from 'solid-js';
import ComponentDemo from '../../components/ComponentDemo';

export const Route = createFileRoute('/components/progressbar')({
  component: ProgressbarPage,
});

function ProgressbarPage() {
  const [progress1, setProgress1] = createSignal(25);
  const [progress2, setProgress2] = createSignal(0);
  const [animatedProgress, setAnimatedProgress] = createSignal(0);

  // Animated progress demo
  let interval: number | undefined;
  const startAnimation = () => {
    setAnimatedProgress(0);
    interval = setInterval(() => {
      setAnimatedProgress(prev => {
        if (prev >= 100) {
          clearInterval(interval);
          return 100;
        }
        return prev + 1;
      });
    }, 50);
  };

  onCleanup(() => {
    if (interval) clearInterval(interval);
  });

  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Progressbar</h1>
        <p class="text-xl text-gray-600">
          Progress indicator component to show completion status of tasks.
        </p>
      </div>

      <ComponentDemo
        title="Basic Progress Bar"
        description="Simple progress bar with different values"
      >
        <div class="space-y-6">
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">25% Complete</span>
              <span class="text-sm text-gray-500">25/100</span>
            </div>
            <Progressbar value={25} />
          </div>
          
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">50% Complete</span>
              <span class="text-sm text-gray-500">50/100</span>
            </div>
            <Progressbar value={50} />
          </div>
          
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">75% Complete</span>
              <span class="text-sm text-gray-500">75/100</span>
            </div>
            <Progressbar value={75} />
          </div>
          
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">Complete!</span>
              <span class="text-sm text-gray-500">100/100</span>
            </div>
            <Progressbar value={100} />
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Interactive Progress Bar"
        description="Progress bar with manual control"
      >
        <div class="space-y-4">
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">Manual Control</span>
              <span class="text-sm text-gray-500">{progress1()}%</span>
            </div>
            <Progressbar value={progress1()} />
          </div>
          
          <div class="flex space-x-2">
            <button
              onClick={() => setProgress1(Math.max(0, progress1() - 10))}
              class="px-3 py-1 bg-red-100 text-red-700 rounded text-sm hover:bg-red-200"
            >
              -10%
            </button>
            <button
              onClick={() => setProgress1(Math.min(100, progress1() + 10))}
              class="px-3 py-1 bg-green-100 text-green-700 rounded text-sm hover:bg-green-200"
            >
              +10%
            </button>
            <button
              onClick={() => setProgress1(0)}
              class="px-3 py-1 bg-gray-100 text-gray-700 rounded text-sm hover:bg-gray-200"
            >
              Reset
            </button>
            <button
              onClick={() => setProgress1(100)}
              class="px-3 py-1 bg-blue-100 text-blue-700 rounded text-sm hover:bg-blue-200"
            >
              Complete
            </button>
          </div>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="Animated Progress"
        description="Progress bar with smooth animation"
      >
        <div class="space-y-4">
          <div>
            <div class="flex justify-between mb-1">
              <span class="text-sm font-medium text-gray-700">Upload Progress</span>
              <span class="text-sm text-gray-500">{animatedProgress()}%</span>
            </div>
            <Progressbar value={animatedProgress()} />
          </div>
          
          <button
            onClick={startAnimation}
            disabled={animatedProgress() > 0 && animatedProgress() < 100}
            class={`px-4 py-2 rounded text-sm font-medium ${
              animatedProgress() > 0 && animatedProgress() < 100
                ? 'bg-gray-200 text-gray-500 cursor-not-allowed'
                : 'bg-indigo-600 text-white hover:bg-indigo-700'
            }`}
          >
            {animatedProgress() > 0 && animatedProgress() < 100 
              ? 'Uploading...' 
              : animatedProgress() === 100 
                ? 'Upload Complete - Click to Restart' 
                : 'Start Upload'
            }
          </button>
        </div>
      </ComponentDemo>

      <ComponentDemo
        title="File Upload Example"
        description="Real-world file upload progress example"
      >
        <div class="space-y-4">
          <div class="border-2 border-dashed border-gray-300 rounded-lg p-6">
            <div class="text-center">
              <svg class="mx-auto h-12 w-12 text-gray-400 mb-4" stroke="currentColor" fill="none" viewBox="0 0 48 48">
                <path d="M28 8H12a4 4 0 00-4 4v20m32-12v8m0 0v8a4 4 0 01-4 4H12a4 4 0 01-4-4v-4m32-4l-3.172-3.172a4 4 0 00-5.656 0L28 28M8 32l9.172-9.172a4 4 0 015.656 0L28 28m0 0l4 4m4-24h8m-4-4v8m-12 4h.02" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
              </svg>
              
              <div class="text-sm text-gray-600 mb-2">
                Uploading: presentation.pptx
              </div>
              <div class="text-xs text-gray-500 mb-3">
                2.3 MB of 4.1 MB
              </div>
              
              <div class="max-w-xs mx-auto">
                <Progressbar value={progress2()} />
              </div>
              
              <div class="text-xs text-gray-500 mt-2">
                {progress2()}% complete • {Math.round((100 - progress2()) / 10)} seconds remaining
              </div>
            </div>
          </div>
          
          <div class="flex space-x-2">
            <input
              type="range"
              min="0"
              max="100"
              value={progress2()}
              onInput={(e) => setProgress2(Number(e.target.value))}
              class="flex-1"
            />
            <span class="text-sm text-gray-600 w-12">{progress2()}%</span>
          </div>
        </div>
      </ComponentDemo>
    </div>
  );
}