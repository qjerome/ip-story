<script setup lang="ts">
import { ref } from "vue";
import PageView from "./PageView.vue";

defineProps<{ msg: string }>();

const count = ref(0);
</script>

<template>
  <PageView>
    <template v-slot:content>
      <div class="hero bg-base-200 h-screen">
        <div class="hero-content text-center">
          <div class="max-w-md">
            <h1 class="text-5xl font-bold">Hello there</h1>
            <p class="py-6">
              Provident cupiditate voluptatem et in. Quaerat fugiat ut assumenda
              excepturi exercitationem quasi. In deleniti eaque aut repudiandae
              et a id nisi.
            </p>
            <button class="btn btn-primary">Get Started</button>
          </div>
        </div>
      </div>
    </template>
  </PageView>
</template>
