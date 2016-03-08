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

	GameObject.prototype.draw = function (context) {
		var text = Math.round(this.value).toString();
		context.fillStyle = this.color;
		context.beginPath();
		context.arc(
			this.pos[0], this.pos[1], this.radius, 0, 2 * Math.PI, true
		);
		context.fill();
		var color;
		if (this.team === "player") {
			color = "blue";
		} else if (this.team === 'cpu'){
			color = "red";
		} else {
			color = "white";
		}
		context.strokeStyle= color; //set the color of the stroke line
  	context.lineWidth = 3;  //define the width of the stroke line
		context.stroke();

		context.fillStyle = "black"; // font color to write the text with
		var font = "bold " + this.radius +"px serif";
		context.font = font;
		// Move it down by half the text height and left by half the text width
		var width = context.measureText(text).width;
		var height = context.measureText("w").width; // this is a GUESS of height
		context.fillText(text, this.pos[0] - (width/2) ,this.pos[1] + (height/2));
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
