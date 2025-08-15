import { createFileRoute } from '@tanstack/solid-router';
import { Carousel } from '../../../../src';
import ComponentDemo from '../../components/ComponentDemo';

export const Route = createFileRoute('/components/carousel')({
  component: CarouselPage,
});

function CarouselPage() {
  return (
    <div class="max-w-4xl">
      <div class="mb-8">
        <h1 class="text-4xl font-bold text-gray-900 mb-4">Carousel</h1>
        <p class="text-xl text-gray-600">
          Interactive carousel component for displaying multiple items with navigation.
        </p>
      </div>

      <ComponentDemo
        title="Basic Carousel"
        description="Carousel with multiple content slides and navigation"
      >
        <Carousel title="Featured Content">
          <div 
            class="min-w-64 h-40 rounded-lg p-6 text-white flex flex-col justify-center"
            style="background: linear-gradient(to right, rgb(59, 130, 246), rgb(147, 51, 234))"
          >
            <h3 class="text-lg font-semibold mb-2">Slide 1</h3>
            <p class="text-sm opacity-90">First slide with some content to demonstrate the carousel functionality.</p>
          </div>
          <div 
            class="min-w-64 h-40 rounded-lg p-6 text-white flex flex-col justify-center"
            style="background: linear-gradient(to right, rgb(34, 197, 94), rgb(59, 130, 246))"
          >
            <h3 class="text-lg font-semibold mb-2">Slide 2</h3>
            <p class="text-sm opacity-90">Second slide showing the scrolling behavior with navigation arrows.</p>
          </div>
          <div 
            class="min-w-64 h-40 rounded-lg p-6 text-white flex flex-col justify-center"
            style="background: linear-gradient(to right, rgb(147, 51, 234), rgb(236, 72, 153))"
          >
            <h3 class="text-lg font-semibold mb-2">Slide 3</h3>
            <p class="text-sm opacity-90">Third slide demonstrating the carousel's smooth scrolling.</p>
          </div>
          <div 
            class="min-w-64 h-40 rounded-lg p-6 text-white flex flex-col justify-center"
            style="background: linear-gradient(to right, rgb(236, 72, 153), rgb(251, 146, 60))"
          >
            <h3 class="text-lg font-semibold mb-2">Slide 4</h3>
            <p class="text-sm opacity-90">Fourth slide to show multiple slides in the carousel.</p>
          </div>
        </Carousel>
      </ComponentDemo>

      <ComponentDemo
        title="Image Gallery Carousel"
        description="Carousel used for displaying a gallery of mock images"
      >
        <Carousel title="Photo Gallery">
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(248, 113, 113), rgb(236, 72, 153))"
          >
            📷 Photo 1
          </div>
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(251, 191, 36), rgb(245, 101, 101))"
          >
            🌅 Photo 2
          </div>
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(34, 197, 94), rgb(59, 130, 246))"
          >
            🌊 Photo 3
          </div>
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(147, 51, 234), rgb(79, 70, 229))"
          >
            🌸 Photo 4
          </div>
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(245, 101, 101), rgb(251, 146, 60))"
          >
            🌺 Photo 5
          </div>
          <div 
            class="min-w-48 h-32 rounded-lg flex items-center justify-center text-white font-medium"
            style="background: linear-gradient(to bottom right, rgb(59, 130, 246), rgb(147, 197, 253))"
          >
            ⛰️ Photo 6
          </div>
        </Carousel>
      </ComponentDemo>

      <ComponentDemo
        title="Product Showcase Carousel"
        description="Carousel for showcasing products or cards"
      >
        <Carousel title="Featured Products">
          <div 
            class="min-w-56 p-4 rounded-lg border"
            style={`background-color: rgb(var(--darkSlate-700)); border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-100))`}
          >
            <div class="h-20 rounded mb-3 flex items-center justify-center text-2xl"
                 style="background: linear-gradient(135deg, rgb(59, 130, 246), rgb(147, 51, 234))">
              💻
            </div>
            <h4 class="font-semibold mb-1">Laptop Pro</h4>
            <p class="text-sm mb-2" style={`color: rgb(var(--lightSlate-300))`}>High-performance laptop for professionals</p>
            <div class="font-bold" style={`color: rgb(var(--primary-400))`}>$1,299</div>
          </div>
          <div 
            class="min-w-56 p-4 rounded-lg border"
            style={`background-color: rgb(var(--darkSlate-700)); border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-100))`}
          >
            <div class="h-20 rounded mb-3 flex items-center justify-center text-2xl"
                 style="background: linear-gradient(135deg, rgb(34, 197, 94), rgb(59, 130, 246))">
              📱
            </div>
            <h4 class="font-semibold mb-1">Smartphone X</h4>
            <p class="text-sm mb-2" style={`color: rgb(var(--lightSlate-300))`}>Latest flagship with advanced features</p>
            <div class="font-bold" style={`color: rgb(var(--primary-400))`}>$899</div>
          </div>
          <div 
            class="min-w-56 p-4 rounded-lg border"
            style={`background-color: rgb(var(--darkSlate-700)); border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-100))`}
          >
            <div class="h-20 rounded mb-3 flex items-center justify-center text-2xl"
                 style="background: linear-gradient(135deg, rgb(236, 72, 153), rgb(251, 146, 60))">
              🎧
            </div>
            <h4 class="font-semibold mb-1">Headphones Pro</h4>
            <p class="text-sm mb-2" style={`color: rgb(var(--lightSlate-300))`}>Noise-cancelling wireless headphones</p>
            <div class="font-bold" style={`color: rgb(var(--primary-400))`}>$299</div>
          </div>
          <div 
            class="min-w-56 p-4 rounded-lg border"
            style={`background-color: rgb(var(--darkSlate-700)); border-color: rgb(var(--darkSlate-600)); color: rgb(var(--lightSlate-100))`}
          >
            <div class="h-20 rounded mb-3 flex items-center justify-center text-2xl"
                 style="background: linear-gradient(135deg, rgb(147, 51, 234), rgb(79, 70, 229))">
              ⌚
            </div>
            <h4 class="font-semibold mb-1">Smart Watch</h4>
            <p class="text-sm mb-2" style={`color: rgb(var(--lightSlate-300))`}>Fitness tracking and smart notifications</p>
            <div class="font-bold" style={`color: rgb(var(--primary-400))`}>$399</div>
          </div>
        </Carousel>
      </ComponentDemo>
    </div>
  );
}