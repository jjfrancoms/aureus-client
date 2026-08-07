package dev.aureus.client.mixin;

import dev.aureus.client.optimization.ParticleBudget;
import net.minecraft.client.particle.Particle;
import net.minecraft.client.particle.ParticleEngine;
import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.At;
import org.spongepowered.asm.mixin.injection.Inject;
import org.spongepowered.asm.mixin.injection.callback.CallbackInfo;

@Mixin(ParticleEngine.class)
abstract class ParticleEngineMixin {
    @Inject(method = "add", at = @At("HEAD"), cancellable = true)
    private void aureus$enforceParticleBudget(Particle particle, CallbackInfo ci) {
        if (!ParticleBudget.accept()) {
            ci.cancel();
        }
    }
}
