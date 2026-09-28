import router from './router'
import store from './store'
import NProgress from 'nprogress'
import 'nprogress/nprogress.css'
import getPageTitle from '@/utils/get-page-title'

NProgress.configure({ showSpinner: false })

// No login: ensure sidebar has constant routes on first paint
if (!store.getters.permission_routes || store.getters.permission_routes.length === 0) {
  store.commit('permission/SET_ROUTES', [])
}

router.beforeEach((to, from, next) => {
  NProgress.start()
  document.title = getPageTitle(to.meta.title)
  next()
})

router.afterEach(() => {
  NProgress.done()
})
