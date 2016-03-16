(function() {
	if (typeof(Oculus) === 'undefined') {
		window.Oculus = {};
	}

	var GameObject = Oculus.GameObject = function (props) {
		this.id = props.id || 0;
		this.pos = props.pos;
		this.vel = props.vel;
		this.team = props.team;
		this.value = props.value;
		this.radius = props.radius;
		this.color = props.color;
		this.game = props.game;
		this.target = props.target || undefined;
		this.startValue = props.startValue || 1;
	};

	GameObject.prototype.draw = function (context, that) {
		var text = Math.round(this.value).toString();
		context.fillStyle = this.color;
		context.beginPath();
		context.arc(
			this.pos[0], this.pos[1], this.radius, 0, 2 * Math.PI, true
		);
		context.fill();
		context.strokeStyle= this.playerColor(); //set the color of the stroke line
  	context.lineWidth = Math.floor(this.value / 5) + 1;  //define the width of the stroke line
		context.stroke();

		context.fillStyle = "black"; // font color to write the text with
		var font =  "bold " + this.radius + "px arial";
		context.font = font;
		// Move it down by half the text height and left by half the text width
		var width = context.measureText(text).width;
		var height = context.measureText("w").width; // this is a GUESS of height
		context.fillText(text, this.pos[0] - (width/2) ,this.pos[1] + (height/2));
		context.font = "bold 24px arial";
		var aiPower;
		if (this.ai) {
			aiPower = this.ai.power;
		} else if(this.game) {
			aiPower = this.game.ai.power;
		} else {
			aiPower = that.ai.power;
		}
		var scoreText =  "Level " + aiPower;
		context.fillText(scoreText, 360, 20);
	};

	GameObject.prototype.playerColor = function () {
		if (this.game && this.game.selectedBase && this.game.selectedBase.id === this.id) {
			return "lightblue";
		} else if (this.team === 'cpu'){
			return "red";
		} else if (this.team === "player")  {
			return 'blue';
		} else {
			return "black";
		}
	};

	GameObject.prototype.hasCollided = function (checkObj) {
		var fromCenter = Oculus.Utils.dist(this.pos, checkObj.pos);
		return fromCenter < (this.radius + checkObj.radius);
	};

	GameObject.prototype.collidedWith = function (base) {
		// TODO: handle collision
	};

	var FRAME_CHANGE = 1000/60;

	GameObject.prototype.move = function (timeChange) {
		var velOffset = timeChange / FRAME_CHANGE;
		var xOffset = this.vel[0] * velOffset;
		var yOffset = this.vel[1] * velOffset;

		this.pos = [this.pos[0] + xOffset, this.pos[1] + yOffset];

	};

	GameObject.prototype.remove = function () {
		this.game.remove(this);
	};

})();
