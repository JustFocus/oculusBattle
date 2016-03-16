(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var Unit = Oculus.Unit = function (params) {
		params.value = params.value;
		params.color = Unit.COLOR;
		params.radius = Unit.RADIUS;
		params.pos = params.pos ||
			params.game.randomPosition(params.basePos);
		params.vel = params.vel ||
			Oculus.Utils.calcVec(params.pos, params.target.pos, Unit.SPEED);
		Oculus.GameObject.call(this, params);
	};

	Unit.COLOR = "#808080";
	Unit.RADIUS = 5;
	Unit.SPEED = 3;
	Unit.VALUE = 1;

	Oculus.Utils.inherits(Unit, Oculus.GameObject);

})();
