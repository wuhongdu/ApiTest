import Vue from 'vue'
import Router from 'vue-router'

Vue.use(Router)

/**
 * constantRoutes — ApiTest: full-screen client shell (Postman-like)
 */
export const constantRoutes = [
  {
    path: '/404',
    component: () => import('@/views/error-page/404'),
    hidden: true
  },
  {
    path: '/',
    component: () => import('@/layout/ClientLayout'),
    redirect: '/workspace',
    children: [
      {
        path: 'workspace',
        component: () => import('@/views/api-test/workspace'),
        name: 'ApiWorkspace',
        meta: { title: 'ApiTest' }
      }
    ]
  },
  { path: '*', redirect: '/404', hidden: true }
]

/**
 * asyncRoutes kept empty for phase 0 (no role-based menus)
 */
export const asyncRoutes = []

const createRouter = () => new Router({
  scrollBehavior: () => ({ y: 0 }),
  routes: constantRoutes
})

const router = createRouter()

export function resetRouter() {
  const newRouter = createRouter()
  router.matcher = newRouter.matcher
}

export default router
