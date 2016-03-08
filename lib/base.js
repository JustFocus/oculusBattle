(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Base = Oculus.Base = function (params) {
		// params.team = params.team;
		// params.radius = params.radius;
		// params.pos = params.pos;
		// params.value = params.value;
		params.color = Base.COLOR;
		params.vel = Base.VEL;
		Oculus.GameObject.call(this, params);
	};

	Base.COLOR = "#808080";
	Base.VEL = 0;

	Oculus.Utils.inherits(Base, Oculus.GameObject);

	Base.prototype.spawnUnits = function (destBase) {
		var troopCount = Math.floor(this.value / 2);
		this.value = this.value - troopCount;

		for (var i = 0; i < troopCount; i++) {
			var unit = new Oculus.Unit({
				team: this.team,
				pos: this.randomPos(),
				target: destBase,
				value: 1
			});

			this.game.add(unit);
		}
	};

	Base.prototype.randomPos = function () {
		// random loc around current base
		var maxDistRadius = this.radius + 10;
		var distFromRaidus =  Math.floor(
			Math.random() * (maxDistRadius - this.radius + 1 )) +
			this.radius;
		var deg = 2 * Math.PI * Math.random();
		return ([distFromRaidus * Math.sin(deg) + this.pos[0],
			distFromRaidus * Math.cos(deg) + this.pos[1]
		]);
	};


})();
