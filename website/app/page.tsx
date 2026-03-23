import Navbar from './components/Navbar'
import Hero from './components/Hero'
import { Marquee } from './components/Marquee'
import ProductDemo from './components/ProductDemo'
import VaporizeStats from './components/VaporizeStats'
import FeaturesGrid from './components/FeaturesGrid'
import BentoCapabilities from './components/BentoCapabilities'
import Compare from './components/Compare'
import UseCases from './components/UseCases'
import HowItWorks from './components/HowItWorks'
import LampCTA from './components/LampCTA'
import FAQ from './components/FAQ'
import Download from './components/Download'
import Changelog from './components/Changelog'
import Rubric from './components/Rubric'
import Footer from './components/Footer'

export default function Home() {
  return (
    <main className="min-h-screen" style={{ background: '#080808' }}>
      <Navbar />
      <Hero />
      <Marquee />
      <ProductDemo />
      <VaporizeStats />
      <FeaturesGrid />
      <BentoCapabilities />
      <Compare />
      <UseCases />
      <HowItWorks />
      <LampCTA />
      <FAQ />
      <div id="download">
        <Download />
      </div>
      <Changelog />
      <Rubric />
      <Footer />
    </main>
  )
}
