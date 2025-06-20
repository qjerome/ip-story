import { createRouter, createWebHistory } from "vue-router";
import HomeView from "./views/HomeView.vue";

export const ROUTE_NAMES = {
  HOME: "home",
  OPENAPI: "openapi",
};

const router = createRouter({
  history: createWebHistory(import.meta.env.BASE_URL),
  routes: [
    {
      path: "/",
      name: ROUTE_NAMES.HOME,
      component: HomeView,
    },

    {
      path: "/openapi",
      name: ROUTE_NAMES.OPENAPI,
      // route level code-splitting
      // this generates a separate chunk (About.[hash].js) for this route
      // which is lazy-loaded when the route is visited.
      component: () => import("./views/OpenApiView.vue"),
    },
    // Catch-all route (404)
    {
      path: "/:pathMatch(.*)*",
      name: "not-found",
      component: () => import("./views/NotFoundView.vue"),
    },
  ],
});

export default router;
