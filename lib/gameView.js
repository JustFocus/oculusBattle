(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var GameView = Oculus.GameView = function(game, context) {
		this.context = context;
		this.game = game;
	};


	GameView.prototype.start = function () {
		this.lastTime = 0;
		requestAnimationFrame(this.animate.bind(this));
	};

	GameView.prototype.animate = function (time) {
		var timeDelta = time - this.lastTime;
		this.game.step(timeDelta);
		this.game.draw(this.context);
		this.lastTime = time;

		requestAnimationFrame(this.animate.bind(this));
	};

})();
